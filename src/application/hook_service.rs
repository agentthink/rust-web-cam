use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use crate::context::{InfraContext, MediaContext};
use crate::infrastructure::DbRepository;
use crate::infrastructure::RedisStore;
use crate::infrastructure::cluster::ClusterManager;
use crate::error::{AppError, Result};
use crate::application::RecordingService;

/// Hook 处理结果
#[derive(Debug, Clone)]
pub enum HookAction {
    /// 允许访问
    Allow,
    /// 拒绝访问（带原因）
    Deny(String),
}

/// Hook 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum HookEvent {
    /// 播放事件
    #[serde(rename = "on_play")]
    OnPlay {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "params", default)]
        params: Option<String>,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
    },

    /// 心跳事件
    #[serde(rename = "on_keepalive")]
    OnKeepalive {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: Option<String>,
        #[serde(rename = "ip")]
        ip: Option<String>,
    },

    /// 停止播放事件
    #[serde(rename = "on_stop")]
    OnStop {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "params", default)]
        params: Option<String>,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
    },

    /// 推流事件
    #[serde(rename = "on_publish")]
    OnPublish {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
        #[serde(rename = "params", default)]
        params: Option<String>,
    },

    /// RTCP 统计事件
    #[serde(rename = "on_rtcp_stats")]
    OnRtcpStats {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        #[serde(rename = "bytesSent")]
        bytes_sent: Option<u64>,
        #[serde(rename = "bytesReceived")]
        bytes_received: Option<u64>,
    },

    /// 录制完成事件（MP4/TS/HLS 等）
    #[serde(rename = "on_record_mp4")]
    #[serde(alias = "on_record_ts")]
    #[serde(alias = "on_record_flv")]
    #[serde(alias = "on_record_hls")]
    OnRecordDone {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        #[serde(rename = "app")]
        app: String,
        #[serde(rename = "file_path")]
        file_path: Option<String>,
        #[serde(rename = "file_size")]
        file_size: Option<u64>,
        #[serde(rename = "time_len")]
        time_len: Option<f64>,
    },

    /// 流量上报事件
    #[serde(rename = "on_flow_report")]
    OnFlowReport {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "app")]
        app: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u32>,
        #[serde(rename = "params")]
        params: Option<String>,
        #[serde(rename = "duration")]
        duration: Option<u32>,
        #[serde(rename = "totalBytes")]
        total_bytes: Option<u64>,
        #[serde(rename = "player")]
        player: Option<bool>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
    },

    /// HTTP 访问事件
    #[serde(rename = "on_http_access")]
    OnHttpAccess {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "params")]
        params: Option<String>,
        #[serde(rename = "path")]
        path: Option<String>,
        #[serde(rename = "is_dir")]
        is_dir: Option<bool>,
        #[serde(flatten)]
        headers: HashMap<String, String>,
    },

    /// RTSP 鉴权领域查询
    #[serde(rename = "on_rtsp_realm")]
    OnRtspRealm {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        app: Option<String>,
        ip: Option<String>,
        port: Option<u16>,
        params: Option<String>,
        schema: Option<String>,
        vhost: Option<String>,
    },

    /// RTSP 鉴权事件
    #[serde(rename = "on_rtsp_auth")]
    OnRtspAuth {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        app: Option<String>,
        ip: Option<String>,
        port: Option<u16>,
        params: Option<String>,
        schema: Option<String>,
        vhost: Option<String>,
        realm: Option<String>,
        user_name: Option<String>,
        must_no_encrypt: Option<bool>,
    },

    /// Shell 登录事件
    #[serde(rename = "on_shell_login")]
    OnShellLogin {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        ip: Option<String>,
        port: Option<u16>,
        passwd: Option<String>,
        user_name: Option<String>,
    },

    /// 流变更事件（注册/注销）
    #[serde(rename = "on_stream_changed")]
    OnStreamChanged {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        app: String,
        stream: String,
        vhost: Option<String>,
        schema: Option<String>,
        regist: bool,
        origin_type: Option<u8>,
        origin_type_str: Option<String>,
        origin_url: Option<String>,
        reader_count: Option<u32>,
        total_reader_count: Option<u32>,
        alive_second: Option<u32>,
        bytes_speed: Option<u64>,
        tracks: Option<Vec<StreamTrackInfo>>,
    },

    /// 流无人观看事件
    #[serde(rename = "on_stream_none_reader")]
    OnStreamNoneReader {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        app: String,
        stream: String,
        vhost: Option<String>,
        schema: Option<String>,
    },

    /// 流未找到事件
    #[serde(rename = "on_stream_not_found")]
    OnStreamNotFound {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(rename = "stream")]
        stream_id: String,
        app: Option<String>,
        ip: Option<String>,
        port: Option<u16>,
        params: Option<String>,
        schema: Option<String>,
        vhost: Option<String>,
    },

    /// 服务器启动事件
    #[serde(rename = "on_server_started")]
    OnServerStarted {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        #[serde(flatten)]
        config: HashMap<String, serde_json::Value>,
    },

    /// 服务器心跳事件
    #[serde(rename = "on_server_keepalive")]
    OnServerKeepalive {
        #[serde(rename = "mediaServerId")]
        media_server_id: String,
        data: Option<ServerKeepaliveData>,
    },

    /// RTP 服务器超时事件
    #[serde(rename = "on_rtp_server_timeout")]
    OnRtpServerTimeout {
        media_server_id: String,
        local_port: Option<u16>,
        re_use_port: Option<bool>,
        ssrc: Option<u32>,
        stream_id: Option<String>,
        tcp_mode: Option<u8>,
    },
}

/// 流轨道信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTrackInfo {
    pub codec_type: Option<u8>,
    pub codec_id: Option<u8>,
    pub codec_id_name: Option<String>,
    pub ready: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<u32>,
    pub sample_rate: Option<u32>,
    pub sample_bit: Option<u16>,
    pub channels: Option<u8>,
}

/// 服务器心跳数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerKeepaliveData {
    pub buffer: Option<u32>,
    pub buffer_like_string: Option<u32>,
    pub buffer_list: Option<u32>,
    pub buffer_raw: Option<u32>,
    pub frame: Option<u32>,
    pub frame_imp: Option<u32>,
    pub media_source: Option<u32>,
    pub multi_media_source_muxer: Option<u32>,
    pub rtmp_packet: Option<u32>,
    pub rtp_packet: Option<u32>,
    pub socket: Option<u32>,
    pub tcp_client: Option<u32>,
    pub tcp_server: Option<u32>,
    pub tcp_session: Option<u32>,
    pub udp_server: Option<u32>,
    pub udp_session: Option<u32>,
}

/// Hook 服务
///
/// 处理来自媒体服务器的 WebHook 回调。
/// 负责 Token 验证、会话创建、流量统计等。
///
/// # 依赖
/// - DbRepository: 数据库操作
/// - RedisStore: Redis 缓存
/// - ClusterManager: 集群管理
/// - session_expiry_secs: 会话过期时间
/// - recording_service: 录制服务
pub struct HookService {
    /// 数据库仓库
    repo: Arc<DbRepository>,
    /// Redis 存储
    redis: Arc<RedisStore>,
    /// 集群管理器
    cluster: Arc<ClusterManager>,
    /// 会话过期时间（秒）
    session_expiry_secs: i64,
    /// 录制服务
    recording_service: Arc<RecordingService>,
}

impl HookService {
    /// 创建 Hook 服务
    ///
    /// # 参数
    /// * `repo` - 数据库仓库
    /// * `redis` - Redis 存储
    /// * `cluster` - 集群管理器
    /// * `session_expiry_secs` - 会话过期时间
    pub fn new(
        repo: Arc<DbRepository>,
        redis: Arc<RedisStore>,
        cluster: Arc<ClusterManager>,
        session_expiry_secs: i64,
        recording_service: Arc<RecordingService>,
    ) -> Self {
        Self {
            repo,
            redis,
            cluster,
            session_expiry_secs,
            recording_service,
        }
    }

    /// 从 InfraContext 和 MediaContext 创建 Hook 服务
    ///
    /// 便捷构造函数，自动从上下文中提取所需组件。
    pub fn from_context(
        infra: &InfraContext,
        media: &MediaContext,
        recording_service: Arc<RecordingService>,
    ) -> Self {
        Self {
            repo: infra.db.clone(),
            redis: infra.redis.clone(),
            cluster: media.cluster.clone(),
            session_expiry_secs: infra.config().session.expiration_secs,
            recording_service,
        }
    }

    /// 处理 Hook 事件
    ///
    /// # 参数
    /// * `event` - Hook 事件
    ///
    /// # 返回
    /// HookAction::Allow 或 HookAction::Deny
    pub async fn handle(&self, event: HookEvent) -> HookAction {
        match event {
            HookEvent::OnPlay {
                media_server_id,
                stream_id,
                ip,
                port: _,
                schema: _,
                params,
                app,
                vhost: _,
            } => {
                self.on_play(&media_server_id, &stream_id, ip.as_deref(), params.as_deref(), app.as_deref())
                    .await
            }

            HookEvent::OnKeepalive {
                media_server_id,
                stream_id,
                ip,
            } => {
                self.on_keepalive(&media_server_id, stream_id.as_deref(), ip.as_deref())
                    .await;
                HookAction::Allow
            }

            HookEvent::OnStop {
                media_server_id,
                stream_id,
                ip,
                port: _,
                schema: _,
                params,
                app: _,
                vhost: _,
            } => {
                self.on_stop(&media_server_id, &stream_id, ip.as_deref(), params.as_deref())
                    .await;
                HookAction::Allow
            }

            HookEvent::OnPublish {
                media_server_id,
                stream_id,
                ip,
                app,
                vhost: _,
                params,
            } => {
                self.on_publish(&media_server_id, &stream_id, ip.as_deref(), app.as_deref(), params.as_deref())
                    .await;
                HookAction::Allow
            }

            HookEvent::OnRtcpStats {
                media_server_id,
                stream_id,
                bytes_sent,
                bytes_received,
            } => {
                self.on_rtcp_stats(&media_server_id, &stream_id, bytes_sent, bytes_received)
                    .await;
                HookAction::Allow
            }

            HookEvent::OnRecordDone {
                stream_id,
                file_path,
                file_size,
                time_len,
                ..
            } => {
                self.on_record_done(&stream_id, file_path.as_deref(), file_size, time_len)
                    .await;
                HookAction::Allow
            }

            HookEvent::OnFlowReport {
                media_server_id,
                stream,
                ip,
                port,
                params,
                duration,
                total_bytes,
                player,
                ..
            } => {
                self.on_flow_report(
                    &media_server_id,
                    &stream,
                    ip.as_deref(),
                    port,
                    params.as_deref(),
                    duration,
                    total_bytes,
                    player,
                )
                .await;
                HookAction::Allow
            }

            HookEvent::OnHttpAccess {
                media_server_id,
                ip,
                port,
                path,
                is_dir,
                ..
            } => {
                self.on_http_access(
                    &media_server_id,
                    ip.as_deref(),
                    port,
                    path.as_deref(),
                    is_dir,
                )
                .await
            }

            HookEvent::OnRtspRealm {
                media_server_id,
                stream_id,
                ip,
                ..
            } => {
                self.on_rtsp_realm(&media_server_id, &stream_id, ip.as_deref())
                    .await
            }

            HookEvent::OnRtspAuth {
                media_server_id,
                stream_id,
                ip,
                user_name,
                must_no_encrypt,
                ..
            } => {
                self.on_rtsp_auth(
                    &media_server_id,
                    &stream_id,
                    ip.as_deref(),
                    user_name.as_deref(),
                    must_no_encrypt.unwrap_or(false),
                )
                .await
            }

            HookEvent::OnShellLogin {
                media_server_id,
                ip,
                passwd,
                user_name,
                ..
            } => {
                self.on_shell_login(
                    &media_server_id,
                    ip.as_deref(),
                    passwd.as_deref(),
                    user_name.as_deref(),
                )
                .await
            }

            HookEvent::OnStreamChanged {
                media_server_id,
                app,
                stream,
                regist,
                origin_type,
                origin_type_str,
                reader_count,
                total_reader_count,
                alive_second,
                bytes_speed,
                tracks,
                ..
            } => {
                self.on_stream_changed(
                    &media_server_id,
                    &app,
                    &stream,
                    regist,
                    origin_type,
                    origin_type_str.as_deref(),
                    reader_count,
                    total_reader_count,
                    alive_second,
                    bytes_speed,
                    tracks.as_ref(),
                )
                .await;
                HookAction::Allow
            }

            HookEvent::OnStreamNoneReader {
                media_server_id,
                app,
                stream,
                ..
            } => {
                self.on_stream_none_reader(&media_server_id, &app, &stream)
                    .await;
                HookAction::Allow
            }

            HookEvent::OnStreamNotFound {
                media_server_id,
                stream_id,
                ip,
                ..
            } => {
                self.on_stream_not_found(&media_server_id, &stream_id, ip.as_deref())
                    .await
            }

            HookEvent::OnServerStarted {
                media_server_id,
                config,
            } => {
                self.on_server_started(&media_server_id, &config).await;
                HookAction::Allow
            }

            HookEvent::OnServerKeepalive {
                media_server_id,
                data,
            } => {
                self.on_server_keepalive(&media_server_id, data.as_ref())
                    .await;
                HookAction::Allow
            }

            HookEvent::OnRtpServerTimeout {
                media_server_id,
                stream_id,
                ..
            } => {
                self.on_rtp_server_timeout(&media_server_id, stream_id.as_deref())
                    .await;
                HookAction::Allow
            }
        }
    }

    /// 处理播放事件
    ///
    /// 验证 Token，创建会话记录，并更新流观看人数。
    async fn on_play(
        &self,
        media_server_id: &str,
        stream_id: &str,
        client_ip: Option<&str>,
        params: Option<&str>,
        _app: Option<&str>,
    ) -> HookAction {
        tracing::info!(
            "[Hook] on_play: server={}, stream={}, ip={:?}",
            media_server_id,
            stream_id,
            client_ip
        );

        // 解析 params 为 HashMap (格式: token=xxx&key=value)
        let params_map: HashMap<String, String> = params
            .map(|p| {
                p.split('&')
                    .filter_map(|pair| {
                        let mut parts = pair.split('=');
                        match (parts.next(), parts.next()) {
                            (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                            _ => None,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 提取 Token
        let token = params_map.get("token").map(|s| s.as_str());

        let token = match token {
            Some(t) => t,
            None => {
                tracing::warn!("[Hook] on_play: no token provided for stream={}", stream_id);
                return HookAction::Deny("Missing token".to_string());
            }
        };

        // 验证 Token
        let stream = match self.get_stream_by_token(token).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!("[Hook] on_play: invalid token for stream={}", stream_id);
                return HookAction::Deny("Invalid token".to_string());
            }
            Err(e) => {
                tracing::error!("[Hook] on_play: failed to get stream by token: {}", e);
                return HookAction::Deny("Internal error".to_string());
            }
        };

        // 更新流观看人数
        let stream_key = crate::domain::stream::make_stream_key(stream.device_tag.as_deref().unwrap_or(""), stream.channel_tag.as_deref().unwrap_or(""));
        if let Err(e) = self.increment_stream_viewers(&stream_key).await {
            tracing::warn!("[Hook] on_play: failed to increment viewers: {}", e);
        }

        // 创建会话
        let mut session = crate::domain::Session::new(
            crate::domain::SessionType::Play,
            0,
        );
        session.state = crate::domain::SessionState::Active;
        session.client_ip = client_ip.map(String::from);
        session.media_server_tag = Some(media_server_id.to_string());
        session.device_tag = stream.device_tag.clone();
        session.channel_tag = stream.channel_tag.clone();
        session.refresh_expiry(self.session_expiry_secs);

        if let Err(e) = self.repo.create_session(&session).await {
            tracing::error!("[Hook] on_play: failed to create session: {}", e);
            return HookAction::Deny("Failed to create session".to_string());
        }

        self.repo.sessions_cache().insert(session.id, session);
        tracing::info!("[Hook] on_play: session created, stream={}, viewers now={}",
            stream_id,
            stream.viewer_count + 1
        );

        HookAction::Allow
    }

    /// 处理停止事件
    ///
    /// 终止会话，更新流观看人数（最后一个观看者离开时触发流清理）。
    async fn on_stop(
        &self,
        media_server_id: &str,
        stream_id: &str,
        client_ip: Option<&str>,
        _params: Option<&str>,
    ) {
        tracing::info!(
            "[Hook] on_stop: server={}, stream={}, ip={:?}",
            media_server_id,
            stream_id,
            client_ip
        );

        let sessions = self.find_sessions_by_stream(stream_id).await;
        let count = sessions.len();
        for session in sessions {
            let mut s = session;
            s.terminate();
            self.repo.sessions_cache().insert(s.id, s);
        }

        if count > 0 {
            tracing::info!(
                "[Hook] on_stop: {} session(s) terminated, stream={}",
                count,
                stream_id
            );
        }

        if let Err(e) = self.decrement_stream_viewers(stream_id).await {
            tracing::warn!("[Hook] on_stop: failed to decrement viewers: {}", e);
        }
    }

    /// 处理心跳事件
    ///
    /// 更新会话活跃时间，同时更新流的最后心跳时间。
    async fn on_keepalive(
        &self,
        media_server_id: &str,
        stream_id: Option<&str>,
        _client_ip: Option<&str>,
    ) {
        tracing::debug!(
            "[Hook] on_keepalive: server={}, stream={:?}",
            media_server_id,
            stream_id
        );

        let Some(stream_id) = stream_id else {
            return;
        };

        let sessions = self.find_sessions_by_stream(stream_id).await;
        for session in sessions {
            let mut s = session;
            s.touch();
            s.refresh_expiry(self.session_expiry_secs);
            self.repo.sessions_cache().insert(s.id, s);
        }

        if let Err(e) = self.touch_stream_keepalive(stream_id).await {
            tracing::debug!("[Hook] on_keepalive: failed to update stream keepalive: {}", e);
        }

        const STREAM_TTL_SECS: u64 = 86400;
        if let Err(e) = self.redis.expire_key(&format!("stream:{}", stream_id), STREAM_TTL_SECS).await {
            tracing::debug!("[Hook] on_keepalive: failed to refresh stream TTL: {}", e);
        }

        if let Some(stream) = self.find_stream_by_stream_key(stream_id) {
            if let Some(ref device_tag) = stream.device_tag {
                if let Some(device) = self.repo.devices_cache().get(device_tag) {
                    let mut device = (*device).clone();
                    device.set_online();
                    if let Err(e) = self.repo.update_device(&device).await {
                        tracing::debug!("[Hook] on_keepalive: failed to update device online status: {}", e);
                    }
                }
            }
        }
    }

    /// 处理推流事件
    ///
    /// 当 RTMP/RTSP 推送流到达媒体服务器时，在本地记录流状态。
    async fn on_publish(
        &self,
        media_server_id: &str,
        stream_id: &str,
        client_ip: Option<&str>,
        _app: Option<&str>,
        _params: Option<&str>,
    ) {
        tracing::info!(
            "[Hook] on_publish: server={}, stream={}, ip={:?}",
            media_server_id,
            stream_id,
            client_ip
        );

        if let Err(e) = self.register_push_stream(media_server_id, stream_id).await {
            tracing::warn!("[Hook] on_publish: failed to register push stream: {}", e);
        }
    }

    /// 处理 RTCP 统计事件
    ///
    /// 更新会话流量统计，同时更新流带宽数据。
    async fn on_rtcp_stats(
        &self,
        media_server_id: &str,
        stream_id: &str,
        bytes_sent: Option<u64>,
        bytes_received: Option<u64>,
    ) {
        tracing::debug!(
            "[Hook] on_rtcp_stats: server={}, stream={}, sent={:?}, recv={:?}",
            media_server_id,
            stream_id,
            bytes_sent,
            bytes_received
        );

        let sessions = self.find_sessions_by_stream(stream_id).await;
        for session in sessions {
            let mut s = session;
            s.update_stats(
                bytes_sent.unwrap_or(0),
                bytes_received.unwrap_or(0),
            );
            self.repo.sessions_cache().insert(s.id, s);
        }

        if let Err(e) = self.update_stream_bandwidth(
            stream_id,
            bytes_received.unwrap_or(0),
            bytes_sent.unwrap_or(0),
        ).await {
            tracing::debug!("[Hook] on_rtcp_stats: failed to update stream bandwidth: {}", e);
        }
    }

    async fn on_record_done(
        &self,
        stream_id: &str,
        file_path: Option<&str>,
        file_size: Option<u64>,
        time_len: Option<f64>,
    ) {
        tracing::info!(
            "[Hook] on_record_done: stream={}, file_path={:?}, file_size={:?}, time_len={:?}",
            stream_id, file_path, file_size, time_len
        );

        let filename = file_path
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .map(String::from)
            .unwrap_or_else(|| stream_id.to_string());

        let size = file_size.unwrap_or(0);
        let duration = time_len.map(|t| t as u64).unwrap_or(0);

        if let Err(e) = self.recording_service.finalize_recording(stream_id, filename, size, duration).await {
            tracing::error!("[Hook] on_record_done: failed to finalize recording: {}", e);
        }
    }

    /// 根据 Token 获取流信息
    async fn get_stream_by_token(
        &self,
        token: &str,
    ) -> std::result::Result<Option<crate::domain::Stream>, AppError> {
        self.repo.get_stream_by_token(token).await
    }

    /// 根据 stream_key 查找流
    fn find_stream_by_stream_key(&self, stream_key: &str) -> Option<crate::domain::Stream> {
        let cache_key = stream_key.replace('_', "/");
        self.repo.streams_cache()
            .get(&cache_key)
            .map(|s| s.clone())
    }

    /// 增加流观看人数
    async fn increment_stream_viewers(&self, stream_key: &str) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.increment_viewers();
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    /// 减少流观看人数
    async fn decrement_stream_viewers(&self, stream_key: &str) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.decrement_viewers();
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    /// 更新流心跳时间
    async fn touch_stream_keepalive(&self, stream_key: &str) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.update_keepalive();
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    /// 更新流带宽数据
    async fn update_stream_bandwidth(&self, stream_key: &str, bytes_in: u64, bytes_out: u64) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.update_bandwidth(bytes_in, bytes_out);
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    /// 注册推送流（RTMP/RTSP push）
    ///
    /// 尝试从 stream_id 匹配设备（stream_key 或 push_urls），若匹配则关联 device_id。
    async fn register_push_stream(&self, media_server_id: &str, stream_id: &str) -> Result<()> {
        let device_id = self.find_device_by_push_stream(stream_id);

        if let Some(s) = self.find_stream_by_stream_key(stream_id) {
            let mut s = s;
            s.update_keepalive();
            if s.state == crate::domain::StreamState::Idle || s.state == crate::domain::StreamState::Error {
                s.start();
            }
            self.repo.update_stream(&s).await?;
            tracing::info!("[Hook] on_publish: updated push stream {} (device_tag={})", stream_id, s.device_tag.as_deref().unwrap_or("unknown"));
        } else {
            let token = uuid::Uuid::new_v4().to_string();
            let (device_tag, channel_tag) = crate::domain::parse_stream_key(stream_id)
                .unwrap_or((stream_id.to_string(), stream_id.to_string()));
            let app = device_id
                .as_ref()
                .and_then(|id| self.repo.devices_cache().get(id))
                .map(|d| d.app.clone().unwrap_or_else(|| "live".to_string()))
                .unwrap_or_else(|| "live".to_string());
            let mut stream = crate::domain::Stream::new(
                media_server_id.to_string(),
                device_tag,
                channel_tag,
                app,
                token,
            );
            stream.start();
            self.repo.create_stream(&stream).await?;
            tracing::info!("[Hook] on_publish: registered push stream {} on server {} (device_tag={})",
                stream_id, media_server_id, device_id.as_deref().unwrap_or("unknown"));
        }

        if let Some(ref did) = device_id {
            if let Some(device) = self.repo.devices_cache().get(did) {
                let mut device = (*device).clone();
                device.set_online();
                self.repo.update_device(&device).await?;
                tracing::info!("[Hook] on_publish: device {} is now online", did);
            }
        }

        Ok(())
    }

    /// 根据推送流的 stream_id 查找关联的设备 tag
    fn find_device_by_push_stream(&self, stream_id: &str) -> Option<String> {
        for entry in self.repo.devices_cache().iter() {
            let device = entry.value();
            for pu in &device.push_urls {
                if !pu.url.is_empty() && (pu.url.contains(stream_id) || stream_id.contains(&device.device_tag.clone().unwrap_or_default())) {
                    return device.device_tag.clone();
                }
            }
        }
        
        let cache_key = stream_id.replace('_', "/");
        self.repo.streams_cache()
            .get(&cache_key)
            .and_then(|s| s.device_tag.clone())
    }

    /// 根据流标识查找活跃会话
    async fn find_sessions_by_stream(
        &self,
        stream_id: &str,
    ) -> Vec<crate::domain::Session> {
        use crate::domain::SessionState;

        self.repo.sessions_cache()
            .iter()
            .filter(|s| {
                s.stream_key() == stream_id
                    && s.state != SessionState::Terminated
                    && s.state != SessionState::Terminating
            })
            .map(|s| s.clone())
            .collect()
    }

    async fn on_flow_report(
        &self,
        media_server_id: &str,
        stream: &str,
        ip: Option<&str>,
        port: Option<u32>,
        params: Option<&str>,
        duration: Option<u32>,
        total_bytes: Option<u64>,
        player: Option<bool>,
    ) {
        tracing::info!(
            "[Hook] on_flow_report: server={}, stream={}, ip={:?}, duration={:?}, bytes={:?}, player={:?}",
            media_server_id, stream, ip, duration, total_bytes, player
        );
    }

    async fn on_http_access(
        &self,
        media_server_id: &str,
        ip: Option<&str>,
        port: Option<u16>,
        path: Option<&str>,
        is_dir: Option<bool>,
    ) -> HookAction {
        tracing::info!(
            "[Hook] on_http_access: server={}, ip={:?}, path={:?}, is_dir={:?}",
            media_server_id, ip, path, is_dir
        );
        HookAction::Allow
    }

    async fn on_rtsp_realm(
        &self,
        media_server_id: &str,
        stream_id: &str,
        ip: Option<&str>,
    ) -> HookAction {
        tracing::info!(
            "[Hook] on_rtsp_realm: server={}, stream={}, ip={:?}",
            media_server_id, stream_id, ip
        );
        HookAction::Allow
    }

    async fn on_rtsp_auth(
        &self,
        media_server_id: &str,
        stream_id: &str,
        ip: Option<&str>,
        user_name: Option<&str>,
        must_no_encrypt: bool,
    ) -> HookAction {
        tracing::info!(
            "[Hook] on_rtsp_auth: server={}, stream={}, ip={:?}, user={:?}, must_no_encrypt={}",
            media_server_id, stream_id, ip, user_name, must_no_encrypt
        );
        HookAction::Allow
    }

    async fn on_shell_login(
        &self,
        media_server_id: &str,
        ip: Option<&str>,
        passwd: Option<&str>,
        user_name: Option<&str>,
    ) -> HookAction {
        tracing::info!(
            "[Hook] on_shell_login: server={}, ip={:?}, user={:?}",
            media_server_id, ip, user_name
        );
        HookAction::Allow
    }

    async fn on_stream_changed(
        &self,
        media_server_id: &str,
        app: &str,
        stream: &str,
        regist: bool,
        origin_type: Option<u8>,
        origin_type_str: Option<&str>,
        reader_count: Option<u32>,
        total_reader_count: Option<u32>,
        alive_second: Option<u32>,
        bytes_speed: Option<u64>,
        tracks: Option<&Vec<StreamTrackInfo>>,
    ) {
        tracing::info!(
            "[Hook] on_stream_changed: server={}, app={}, stream={}, regist={}, origin_type={:?}, readers={:?}/{:?}",
            media_server_id, app, stream, regist, origin_type_str, reader_count, total_reader_count
        );

        if !regist {
            if let Some(s) = self.find_stream_by_stream_key(stream) {
                let mut s = s;
                s.stopped();
                if let Err(e) = self.repo.update_stream(&s).await {
                    tracing::warn!("[Hook] on_stream_changed: failed to stop stream: {}", e);
                }
                tracing::info!("[Hook] on_stream_changed: stream {} stopped", stream);
            }
            return;
        }

        if let Some(s) = self.find_stream_by_stream_key(stream) {
            let mut s = s;
            s.update_keepalive();
            if s.state != crate::domain::StreamState::Active {
                s.start();
            }
            if let Err(e) = self.repo.update_stream(&s).await {
                tracing::warn!("[Hook] on_stream_changed: failed to update stream: {}", e);
            }
        }

        if let Some(device_id) = self.find_device_by_push_stream(stream) {
            if let Some(device) = self.repo.devices_cache().get(&device_id) {
                let mut device = (*device).clone();
                device.set_online();
                if let Err(e) = self.repo.update_device(&device).await {
                    tracing::warn!("[Hook] on_stream_changed: failed to update device online status: {}", e);
                }
            }
        }
    }

    async fn on_stream_none_reader(
        &self,
        media_server_id: &str,
        app: &str,
        stream: &str,
    ) {
        tracing::info!(
            "[Hook] on_stream_none_reader: server={}, app={}, stream={}",
            media_server_id, app, stream
        );
    }

    async fn on_stream_not_found(
        &self,
        media_server_id: &str,
        stream_id: &str,
        ip: Option<&str>,
    ) -> HookAction {
        tracing::info!(
            "[Hook] on_stream_not_found: server={}, stream={}, ip={:?}",
            media_server_id, stream_id, ip
        );
        HookAction::Allow
    }

    async fn on_server_started(
        &self,
        media_server_id: &str,
        config: &std::collections::HashMap<String, serde_json::Value>,
    ) {
        tracing::info!(
            "[Hook] on_server_started: server={}",
            media_server_id
        );
    }

    async fn on_server_keepalive(
        &self,
        media_server_id: &str,
        data: Option<&ServerKeepaliveData>,
    ) {
        tracing::debug!(
            "[Hook] on_server_keepalive: server={}, data={:?}",
            media_server_id, data
        );
    }

    async fn on_rtp_server_timeout(
        &self,
        media_server_id: &str,
        stream_id: Option<&str>,
    ) {
        tracing::info!(
            "[Hook] on_rtp_server_timeout: server={}, stream_id={:?}",
            media_server_id, stream_id
        );
    }
}