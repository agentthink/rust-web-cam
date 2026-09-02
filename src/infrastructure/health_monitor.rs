use std::sync::Arc;
use std::time::Duration;
use crate::adapter::media_server::MediaServerAdapter;
use crate::application::media_server_service::MediaServerService;
use crate::application::StreamRecoveryService;
use crate::domain::traits::CacheStore;
use crate::infrastructure::RedisStore;
use crate::infrastructure::cluster::ClusterManager;

pub const SERVER_KEY_PREFIX: &str = "media_servers";
pub const ONLINE_SET_KEY: &str = "media_servers:online";
pub const HEARTBEAT_TTL_SECS: u64 = 30;

pub struct HealthMonitor {
    redis: Arc<RedisStore>,
    cluster: Arc<ClusterManager>,
    media_server_service: Arc<MediaServerService>,
    stream_recovery_service: Option<Arc<StreamRecoveryService>>,
    interval_secs: u64,
}

impl HealthMonitor {
    pub fn new(
        redis: Arc<RedisStore>,
        cluster: Arc<ClusterManager>,
        media_server_service: Arc<MediaServerService>,
        stream_recovery_service: Option<Arc<StreamRecoveryService>>,
        interval_secs: u64,
    ) -> Self {
        Self {
            redis,
            cluster,
            media_server_service,
            stream_recovery_service,
            interval_secs,
        }
    }

    pub async fn start(self: Arc<Self>) {
        let interval = Duration::from_secs(self.interval_secs);

        tokio::spawn(async move {
            tracing::info!("[HealthMonitor] Started (interval: {}s)", self.interval_secs);
            let cluster_tags = self.cluster.get_all_server_tags().clone();
            for  config_tag in &cluster_tags {
                if let Some(ref svc) = self.stream_recovery_service {
                    tracing::info!("[HealthMonitor] Triggering stream recovery for server {}", config_tag);
                    svc.mark_streams_recovering_for_media_server(&config_tag).await;
                    svc.restart_streams_for_media_server(&config_tag).await;
                }
            }
            loop {
                let start = std::time::Instant::now();
                self.sync_servers().await;
                let elapsed = start.elapsed();
                tracing::debug!("[HealthMonitor] Sync completed in {:?}", elapsed);
                tokio::time::sleep(interval).await;
            }
        });
    }

    async fn sync_servers(&self) {
        let servers = self.media_server_service.list();
        let cluster_tags = self.cluster.get_all_server_tags();
        let cluster_tags_set: std::collections::HashSet<_> = cluster_tags.iter().collect();

        for server in &servers {
            let tag = server.server_tag.clone();

            // 优先复用 ClusterManager 中的 adapter
            let adapter = self.cluster.get_server(&tag)
                .map(|a| Arc::clone(&a))
                .or_else(|| self.create_adapter(server));

            let adapter = match adapter {
                Some(a) => a,
                None => continue,
            };

            // 只调用一次 get_status 获取完整状态
            match adapter.get_status().await {
                Ok(status) => {
                    let in_cluster = cluster_tags_set.contains(&tag);

                    if server.enabled {
                        if !in_cluster {
                            // 在线且启用但不在 cluster 中 → 注册
                            self.cluster.register_server(server.to_config());
                            tracing::info!("[HealthMonitor] Server {} registered to cluster (online)", tag);

                            // 触发该媒体服务器上的流恢复
                            if let Some(ref svc) = self.stream_recovery_service {
                                tracing::info!("[HealthMonitor] Restarting streams on server {}", tag);
                                svc.restart_streams_for_media_server(&tag).await;
                            }
                        }
                        self.update_server_status(&tag, &status).await;
                    } else {
                        // 禁用状态，从 cluster 移除
                        if in_cluster {
                            self.cluster.unregister_server(&tag);
                            tracing::warn!("[HealthMonitor] Server {} is disabled, unregistered from cluster", tag);
                        }
                        self.mark_server_offline(&tag).await;
                    }
                }
                Err(e) => {
                    // 获取状态失败，视为离线
                    tracing::warn!("[HealthMonitor] Server {} get_status failed: {}", tag, e);
                    if cluster_tags_set.contains(&tag) {
                        self.cluster.unregister_server(&tag);
                        tracing::warn!("[HealthMonitor] Server {} unregistered from cluster (offline)", tag);
                    }
                    self.mark_server_offline(&tag).await;
                }
            }
        }
    }

    fn create_adapter(&self, server: &crate::application::media_server_service::ManagedServer) -> Option<Arc<dyn MediaServerAdapter>> {
        let config = server.to_config();
        match config.server_type.as_str() {
            "zlmediakit" => Some(Arc::new(crate::adapter::media_server::zlmediakit::ZlMediaKitAdapter::new(config))),
            "srs" => Some(Arc::new(crate::adapter::media_server::srs::SrsAdapter::new(config))),
            "xiu" => Some(Arc::new(crate::adapter::media_server::xiu::XiuAdapter::new(config))),
            _ => None,
        }
    }

    async fn update_server_status(&self, tag: &str, status: &crate::adapter::media_server::ServerStatus) {
        let key = format!("{}:{}", SERVER_KEY_PREFIX, tag);
        let fields = [
            ("name", status.name.as_str()),
            ("type", status.server_type.as_str()),
            ("online", "true"),
            ("session_count", &status.session_count.to_string()),
            ("cpu_usage", &format!("{:.1}", status.cpu_usage)),
            ("memory_usage", &format!("{:.1}", status.memory_usage)),
            ("bandwidth_in", &status.bandwidth_in.to_string()),
            ("bandwidth_out", &status.bandwidth_out.to_string()),
            ("last_heartbeat", &chrono::Utc::now().timestamp().to_string()),
        ];

        if let Err(e) = self.redis.hmset(&key, &fields).await {
            tracing::error!("[HealthMonitor] Failed to update {}: {}", tag, e);
            return;
        }
        let _ = self.redis.expire(&key, HEARTBEAT_TTL_SECS).await;
        let _ = self.redis.sadd(ONLINE_SET_KEY, tag).await;
        tracing::debug!("[HealthMonitor] Updated server {} (online)", tag);
    }

    async fn mark_server_offline(&self, name: &str) {
        let key = format!("{}:{}", SERVER_KEY_PREFIX, name);
        let _ = self.redis.hset(&key, "online", "false").await;
        let _ = self.redis.srem(ONLINE_SET_KEY, name).await;
        tracing::warn!("[HealthMonitor] Server {} marked offline", name);
    }
}