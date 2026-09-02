use std::sync::Arc;
use crate::config::AppConfig;
use crate::context::{InfraContext, MediaContext};
use crate::infrastructure::cluster::ClusterManager;
use crate::infrastructure::db_repository::DbRepository;
use crate::application::{
    DeviceService, StreamService, SessionService, RecordingService,
    MediaServerService, HookService, PlayerLayoutService,
    PlayService, WsBroadcaster, StreamRecoveryService,
    zlmediakit_hook::ZlMediaKitHookHandler, ChannelService,
};
use crate::domain::traits::{DeviceLookup, StreamManager, EventPublisher, CacheStore};
use crate::protocol::traits::ProtocolDeps;
use crate::protocol::adapter_manager::{AdapterEntry, set_adapter as am_set, remove_adapter as am_remove};
use crate::auth::AuthState;
pub use crate::auth::casbin::CasbinManager;
use crate::auth::auth_db::PostgresAuthRepository;

/// 服务注册中心
///
/// 集中管理所有 Service 实例，负责依赖注入和生命周期管理。
#[derive(Clone)]
pub struct ServiceRegistry {
    /// 基础设施上下文
    pub infra: InfraContext,

    /// 媒体上下文
    pub media: MediaContext,

    /// 设备管理服务
    pub device_service: Arc<DeviceService>,

    /// 流管理服务
    pub stream_service: Arc<StreamService>,

    /// 流恢复服务
    pub stream_recovery_service: Arc<StreamRecoveryService>,

    /// 会话管理服务
    pub session_service: Arc<SessionService>,

    /// 录制管理服务
    pub recording_service: Arc<RecordingService>,

    /// 媒体服务器管理服务
    pub media_server_service: Arc<MediaServerService>,

    /// WebHook 回调服务 (旧版兼容)
    pub hook_service: Arc<HookService>,

    /// ZLMediaKit Hook 处理器
    pub zlmediakit_hook_handler: Arc<ZlMediaKitHookHandler>,

    /// 播放器布局服务
    pub player_layout_service: Arc<PlayerLayoutService>,

    /// 通道服务
    pub channel_service: Arc<ChannelService>,

    /// 播放服务
    pub play_service: Arc<PlayService>,

    /// WebSocket 广播器
    pub ws_broadcaster: Arc<WsBroadcaster>,

    /// 协议层依赖 (Trait objects)
    pub protocol_deps: ProtocolDeps,
}

impl ServiceRegistry {
    /// 创建服务注册中心
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        tracing::info!("[ServiceRegistry] Building service registry...");

        // ── 第1层: 基础设施上下文 ──
        // 先创建 DbRepository（不加载缓存）
        let db = Arc::new(
            DbRepository::new_without_load(&config.database.url, config.database.debug_sql.unwrap_or(false))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?
        );

        // 同步配置服务器到数据库（load_caches 之前）
        for cfg in &config.media_servers.servers {
            let tag = if cfg.server_tag.is_empty() {
                cfg.name.clone()
            } else {
                cfg.server_tag.clone()
            };

            let exists_in_db = db.server_exists_by_tag(&tag).await?;
            if !exists_in_db {
                tracing::info!("[ServiceRegistry] Syncing config server '{}' to DB", tag);
                let new_server = crate::domain::server::Server {
                    id: 0,
                    name: cfg.name.clone(),
                    url: cfg.url.clone(),
                    api_key: cfg.api_key.clone(),
                    server_type: crate::domain::server::ServerType::from_str(&cfg.server_type).unwrap_or(crate::domain::server::ServerType::Zlmediakit),
                    weight: cfg.weight,
                    enabled: cfg.enabled,
                    server_tag: tag.clone(),
                    protocol_ports: cfg.protocol_ports.clone(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                if let Err(e) = db.create_server(&new_server).await {
                    tracing::warn!("[ServiceRegistry] Failed to sync config server to DB: {}", e);
                }
            }
        }

        // 加载缓存
        db.load_caches().await?;

        // 构建 InfraContext
        let infra = InfraContext::from_parts(db.clone(), Arc::new(config)).await?;

        // ── 第2层: 媒体上下文 ──
        let cluster = Arc::new(ClusterManager::new_from_configs(vec![]));

        // 媒体服务器管理服务（从数据库加载）
        let media_server_service = Arc::new(MediaServerService::new(
            infra.db.clone(),
            cluster.clone(),
        ));

        let media = MediaContext::new(&infra, cluster,media_server_service.clone()).await?;

        // ── 第3层: 业务服务 ──

        // 录制服务
        let recording_service = Arc::new(RecordingService::new(
            infra.db.clone(),
            media.cluster.clone(),
        ));

        // 流服务
        let stream_service = Arc::new(StreamService::new(
            infra.clone(),
            media.clone(),
            recording_service.clone(),
        ));

        // 流恢复服务
        let stream_recovery_service = Arc::new(StreamRecoveryService::new(
            infra.clone(),
            media.clone(),
            stream_service.clone(),
        ));

        // 设备服务
        let device_service = Arc::new(DeviceService::new(
            infra.clone(),
            media.clone(),
            stream_service.clone(),
        ));

        // 会话服务
        let session_service = Arc::new(SessionService::new(
            infra.clone(),
            media.clone(),
        ));

        // Hook 服务
        let hook_service = Arc::new(HookService::new(
            infra.db.clone(),
            infra.redis.clone(),
            media.cluster.clone(),
            infra.config().session.expiration_secs,
            recording_service.clone(),
        ));

        // ZLMediaKit Hook 处理器
        let zlm_api_key = media_server_service.list().iter()
            .find(|s| s.server_type == crate::domain::server::ServerType::Zlmediakit)
            .map(|s| s.api_key.clone());
        let zlmediakit_hook_handler = Arc::new(
            ZlMediaKitHookHandler::from_context(
                &infra,
                &media,
                recording_service.clone(),
                zlm_api_key,
            ).with_stream_recovery_service(stream_recovery_service.clone())
        );



        // 播放器布局服务
        let player_layout_service = Arc::new(PlayerLayoutService::new(
            infra.db.clone(),
        ));

        // 通道服务
        let channel_service = Arc::new(ChannelService::new(
            infra.db.clone(),
        ));

        // ── 第4层: 协议依赖 ──
        let register_fn: Arc<dyn Fn(String, AdapterEntry) + Send + Sync> =
            Arc::new(|key, entry| am_set(key, entry));
        let unregister_fn: Arc<dyn Fn(String) + Send + Sync> =
            Arc::new(|key| am_remove(&key));
        let protocol_deps = ProtocolDeps::new(
            device_service.clone() as Arc<dyn DeviceLookup>,
            stream_service.clone() as Arc<dyn StreamManager>,
            infra.event_bus.clone() as Arc<dyn EventPublisher>,
            media.cluster.clone(),
            media.rtp_tunnel.clone(),
            infra.redis.clone() as Arc<dyn CacheStore>,
            infra.config.clone(),
        ).with_registration(register_fn, unregister_fn);

        let ws_broadcaster = infra.ws_broadcaster.clone();
        let play_service = media.play_service.clone();

        tracing::info!("[ServiceRegistry] All services initialized");

        Ok(Self {
            infra,
            media,
            device_service,
            stream_service,
            stream_recovery_service,
            session_service,
            recording_service,
            media_server_service,
            hook_service,
            zlmediakit_hook_handler,
            player_layout_service,
            channel_service,
            play_service,
            ws_broadcaster,
            protocol_deps,
        })
    }

    /// 获取认证状态
    pub async fn get_auth_state(&self) -> anyhow::Result<AuthState> {
        let jwt_config = self.infra.config().jwt.clone();
        let auth_repo = Arc::new(
            PostgresAuthRepository::new(&self.infra.config().database.url).await?
        );
        let casbin = Arc::new(
            CasbinManager::new(auth_repo.clone()).await
                .map_err(|e| anyhow::anyhow!("Casbin init failed: {}", e))?
        );

        Ok(AuthState::new(jwt_config, casbin, auth_repo))
    }

    /// 启动所有后台服务
    pub async fn start_all(&self) {
        tracing::info!("[ServiceRegistry] Starting all services...");
        self.device_service.start().await;
        self.session_service.start(self.infra.config().session.expiration_secs).await;
        self.stream_recovery_service.clone().start().await;
        self.start_health_monitor().await;
        self.start_ws_service().await;
        tracing::info!("[ServiceRegistry] All services started");
    }

    async fn start_health_monitor(&self) {
        let monitor = crate::infrastructure::health_monitor::HealthMonitor::new(
            self.infra.redis.clone(),
            self.media.cluster.clone(),
            self.media_server_service.clone(),
            Some(self.stream_recovery_service.clone()),
            self.infra.config().cluster.health_check_interval_secs,
        );
        let monitor = std::sync::Arc::new(monitor);
        monitor.start().await;
    }

    async fn start_ws_service(&self) {
        let ws_service = crate::application::ws_service::WsService::new(
            self.infra.ws_broadcaster.clone(),
            self.device_service.clone(),
            self.session_service.clone(),
            self.media.cluster.clone(),
        );
        let ws_service = std::sync::Arc::new(ws_service);
        ws_service.start();
    }

    /// 优雅关闭所有服务
    pub async fn graceful_shutdown(&self) {
        tracing::info!("[ServiceRegistry] Initiating graceful shutdown...");
        self.device_service.stop().await;
        self.session_service.stop().await;
        self.media.rtp_tunnel.shutdown().await;
        tracing::info!("[ServiceRegistry] Shutdown complete");
    }

    /// 获取 Dashboard 数据
    pub async fn get_dashboard_data(&self) -> serde_json::Value {
        let device_stats = self.device_service.get_stats();
        let stream_stats = self.stream_service.get_stats().await;
        let servers = self.media_server_service.list();

        let mut server_statuses = Vec::new();
        let mut total_sessions = 0u32;
        let mut online_count = 0usize;

        tracing::debug!("[Dashboard] Fetching status for {} servers", servers.len());
        for server in &servers {
            let status = self.media.cluster.get_server_status(&server.server_tag).await;
            let online = status.as_ref().map(|s| s.online).unwrap_or(false);
            let sessions = status.as_ref().map(|s| s.session_count).unwrap_or(0);

            tracing::debug!("[Dashboard] Server tag='{}', name='{}', online={}", 
                server.server_tag, server.name, online);

            if online { online_count += 1; }
            total_sessions += sessions;

            server_statuses.push(serde_json::json!({
                "id": server.id,
                "name": server.name,
                "server_tag": server.server_tag,
                "server_type": server.server_type,
                "online": online,
                "enabled": server.enabled,
                "session_count": sessions,
            }));
        }

        let server_count = servers.len();
        let health_score = if server_count == 0 { 0.0 } else { (online_count as f64 / server_count as f64) * 100.0 };

        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "devices": device_stats,
            "streams": stream_stats,
            "servers": {
                "total": server_count,
                "online": online_count,
                "offline": server_count.saturating_sub(online_count),
                "servers": server_statuses,
            },
            "health": {
                "score": health_score,
                "level": if health_score >= 80.0 { "healthy" } else if health_score >= 50.0 { "degraded" } else { "critical" },
            }
        })
    }
}