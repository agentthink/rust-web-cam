use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use crate::context::{InfraContext, MediaContext};
use crate::infrastructure::DbRepository;
use crate::infrastructure::RedisStore;
use crate::infrastructure::cluster::ClusterManager;
use crate::error::{AppError, Result};
use crate::application::RecordingService;

/// Hook 基础响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResponse {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_hls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_rtmp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_rtsp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_mp4: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_mute_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mp4_save_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hls_save_path: Option<String>,
}

impl HookResponse {
    pub fn allow() -> Self {
        Self { code: 0, msg: Some("success".to_string()), ..Default::default() }
    }

    pub fn deny(msg: &str) -> Self {
        Self { code: 1, msg: Some(msg.to_string()), ..Default::default() }
    }

    pub fn rtsp_realm(realm: &str) -> Self {
        Self { 
            code: 0, 
            msg: Some("success".to_string()),
            realm: Some(realm.to_string()),
            ..Default::default() 
        }
    }

    pub fn rtsp_auth(encrypted: bool, passwd: &str) -> Self {
        Self {
            code: 0,
            msg: Some("success".to_string()),
            encrypted: Some(encrypted),
            passwd: Some(passwd.to_string()),
            ..Default::default()
        }
    }

    pub fn http_access(path: &str, second: u32) -> Self {
        Self {
            code: 0,
            err: Some(String::new()),
            path: Some(path.to_string()),
            second: Some(second),
            ..Default::default()
        }
    }
}

impl Default for HookResponse {
    fn default() -> Self {
        Self { code: 0, msg: None, err: None, path: None, second: None, close: None, realm: None, encrypted: None, passwd: None, enable_hls: None, enable_rtmp: None, enable_rtsp: None, enable_mp4: None, enable_audio: None, add_mute_audio: None, mp4_save_path: None, hls_save_path: None }
    }
}

/// Hook 处理结果
#[derive(Debug, Clone)]
pub enum HookAction {
    Allow(HookResponse),
    Deny(String),
}

impl HookAction {
    pub fn allow() -> Self {
        HookAction::Allow(HookResponse::allow())
    }
    
    pub fn deny(msg: &str) -> Self {
        HookAction::Deny(msg.to_string())
    }
    
    pub fn to_response(self) -> HookResponse {
        match self {
            HookAction::Allow(r) => r,
            HookAction::Deny(msg) => HookResponse::deny(&msg),
        }
    }
}

/// ZLMediaKit Hook 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum ZlMediaKitHookEvent {
    #[serde(rename = "on_play")]
    OnPlay {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "params")]
        params: Option<String>,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
    },

    #[serde(rename = "on_publish")]
    OnPublish {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
        #[serde(rename = "params")]
        params: Option<String>,
    },

    #[serde(rename = "on_stop")]
    OnStop {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "params")]
        params: Option<String>,
    },

    #[serde(rename = "on_keepalive")]
    OnKeepalive {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: Option<String>,
        #[serde(rename = "ip")]
        ip: Option<String>,
    },

    #[serde(rename = "on_rtcp_stats")]
    OnRtcpStats {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "bytesSent")]
        bytes_sent: Option<u64>,
        #[serde(rename = "bytesReceived")]
        bytes_received: Option<u64>,
    },

    #[serde(rename = "on_record_mp4")]
    #[serde(alias = "on_record_flv")]
    #[serde(alias = "on_record_hls")]
    OnRecordDone {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "app")]
        app: String,
        #[serde(rename = "file_path")]
        file_path: Option<String>,
        #[serde(rename = "file_size")]
        file_size: Option<u64>,
        #[serde(rename = "time_len")]
        time_len: Option<f64>,
    },

    #[serde(rename = "on_record_ts")]
    OnRecordTs {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "app")]
        app: String,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "file_path")]
        file_path: Option<String>,
        #[serde(rename = "file_size")]
        file_size: Option<u64>,
        #[serde(rename = "segment_index")]
        segment_index: Option<u32>,
        #[serde(rename = "segment_duration")]
        segment_duration: Option<f64>,
        #[serde(rename = "miliseconds")]
        miliseconds: Option<u64>,
        #[serde(rename = "start_time")]
        start_time: Option<String>,
        #[serde(rename = "end_time")]
        end_time: Option<String>,
    },

    #[serde(rename = "on_flow_report")]
    OnFlowReport {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
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

    #[serde(rename = "on_http_access")]
    OnHttpAccess {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
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
    },

    #[serde(rename = "on_rtsp_realm")]
    OnRtspRealm {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "params")]
        params: Option<String>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
    },

    #[serde(rename = "on_rtsp_auth")]
    OnRtspAuth {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "params")]
        params: Option<String>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
        #[serde(rename = "realm")]
        realm: Option<String>,
        #[serde(rename = "user_name")]
        user_name: Option<String>,
        #[serde(rename = "must_no_encrypt")]
        must_no_encrypt: Option<bool>,
    },

    #[serde(rename = "on_shell_login")]
    OnShellLogin {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "passwd")]
        passwd: Option<String>,
        #[serde(rename = "user_name")]
        user_name: Option<String>,
    },

    #[serde(rename = "on_stream_changed")]
    OnStreamChanged {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "app")]
        app: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "regist")]
        regist: bool,
        #[serde(rename = "origin_type")]
        origin_type: Option<u8>,
        #[serde(rename = "origin_type_str")]
        origin_type_str: Option<String>,
        #[serde(rename = "reader_count")]
        reader_count: Option<u32>,
        #[serde(rename = "total_reader_count")]
        total_reader_count: Option<u32>,
        #[serde(rename = "alive_second")]
        alive_second: Option<u32>,
        #[serde(rename = "bytes_speed")]
        bytes_speed: Option<u64>,
        #[serde(rename = "tracks")]
        tracks: Option<Vec<ZlMediaKitStreamTrack>>,
    },

    #[serde(rename = "on_stream_none_reader")]
    OnStreamNoneReader {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "app")]
        app: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
        #[serde(rename = "schema")]
        schema: Option<String>,
    },

    #[serde(rename = "on_stream_not_found")]
    OnStreamNotFound {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "stream")]
        stream: String,
        #[serde(rename = "app")]
        app: Option<String>,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "port")]
        port: Option<u16>,
        #[serde(rename = "params")]
        params: Option<String>,
        #[serde(rename = "schema")]
        schema: Option<String>,
        #[serde(rename = "vhost")]
        vhost: Option<String>,
    },

    #[serde(rename = "on_server_started")]
    OnServerStarted {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(flatten)]
        config: HashMap<String, serde_json::Value>,
    },

    #[serde(rename = "on_server_keepalive")]
    OnServerKeepalive {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        data: Option<ZlMediaKitServerKeepaliveData>,
    },

    #[serde(rename = "on_server_exited")]
    OnServerExited {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(flatten)]
        exit_data: HashMap<String, serde_json::Value>,
    },

    #[serde(rename = "on_send_rtp_stopped")]
    OnSendRtpStopped {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        #[serde(rename = "local_port")]
        local_port: Option<u16>,
        #[serde(rename = "ssrc")]
        ssrc: Option<String>,
        #[serde(rename = "stream_id")]
        stream_id: Option<String>,
        #[serde(rename = "ip")]
        ip: Option<String>,
        #[serde(rename = "rtp")]
        rtp: Option<bool>,
    },

    #[serde(rename = "on_rtp_server_timeout")]
    OnRtpServerTimeout {
        #[serde(rename = "mediaServerId")]
        media_server_tag: String,
        local_port: Option<u16>,
        re_use_port: Option<bool>,
        ssrc: Option<u32>,
        stream_id: Option<String>,
        tcp_mode: Option<u8>,
    },
}

/// ZLMediaKit 流轨道信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZlMediaKitStreamTrack {
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

/// ZLMediaKit 服务器心跳数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZlMediaKitServerKeepaliveData {
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

/// ZLMediaKit Hook 处理器
pub struct ZlMediaKitHookHandler {
    repo: Arc<DbRepository>,
    redis: Arc<RedisStore>,
    cluster: Arc<ClusterManager>,
    session_expiry_secs: i64,
    recording_service: Arc<RecordingService>,
    secret: Option<String>,
    stream_recovery_service: Option<Arc<crate::application::StreamRecoveryService>>,
}

impl ZlMediaKitHookHandler {
    pub fn new(
        repo: Arc<DbRepository>,
        redis: Arc<RedisStore>,
        cluster: Arc<ClusterManager>,
        session_expiry_secs: i64,
        recording_service: Arc<RecordingService>,
        secret: Option<String>,
    ) -> Self {
        Self { repo, redis, cluster, session_expiry_secs, recording_service, secret, stream_recovery_service: None }
    }

    pub fn with_stream_recovery_service(mut self, stream_recovery_service: Arc<crate::application::StreamRecoveryService>) -> Self {
        self.stream_recovery_service = Some(stream_recovery_service);
        self
    }

    pub fn from_context(
        infra: &InfraContext,
        media: &MediaContext,
        recording_service: Arc<RecordingService>,
        secret: Option<String>,
    ) -> Self {
        Self::new(
            infra.db.clone(),
            infra.redis.clone(),
            media.cluster.clone(),
            infra.config().session.expiration_secs,
            recording_service,
            secret,
        )
    }

    /// 验证请求是否来自已配置的媒体服务器
    fn validate_media_server(&self, media_server_tag: &str) -> Option<Arc<dyn crate::adapter::media_server::MediaServerAdapter>> {
        self.cluster.get_server(media_server_tag)
    }

    /// 验证 secret（admin_params）
    fn validate_secret(&self, params: Option<&str>) -> bool {
        let Some(secret) = &self.secret else {
            return true;
        };
        
        let params = params.unwrap_or("");
        let expected = format!("secret={}", secret);
        params.contains(&expected)
    }

    /// 解析 query string 为 HashMap
    fn parse_params(params: Option<&str>) -> HashMap<String, String> {
        params
            .map(|p| {
                p.split('&')
                    .filter_map(|pair| {
                        let mut parts = pair.splitn(2, '=');
                        match (parts.next(), parts.next()) {
                            (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                            _ => None,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 处理 hook 事件
    pub async fn handle(&self, event: ZlMediaKitHookEvent) -> HookResponse {
        match event {
            ZlMediaKitHookEvent::OnPlay { media_server_tag, stream, ip, port: _, schema: _, params, app, vhost: _ } => 
                self.on_play(&media_server_tag, &stream, ip.as_ref(), params.as_ref(), app.as_ref()).await,
            ZlMediaKitHookEvent::OnPublish { media_server_tag, stream, ip, app, vhost: _, params } => 
                self.on_publish(&media_server_tag, &stream, ip.as_ref(), app.as_ref(), params.as_ref()).await,
            ZlMediaKitHookEvent::OnStop { media_server_tag, stream, ip, params } => 
                self.on_stop(&media_server_tag, &stream, ip.as_ref(), params.as_ref()).await,
            ZlMediaKitHookEvent::OnKeepalive { media_server_tag, stream, ip } => 
                self.on_keepalive(&media_server_tag, stream.as_ref(), ip.as_ref()).await,
            ZlMediaKitHookEvent::OnRtcpStats { media_server_tag, stream, bytes_sent, bytes_received } => 
                self.on_rtcp_stats(&media_server_tag, &stream, bytes_sent, bytes_received).await,
            ZlMediaKitHookEvent::OnRecordDone { media_server_tag, stream, app, file_path, file_size, time_len } => 
                self.on_record_done(&media_server_tag, &stream, &app, file_path.as_ref(), file_size, time_len).await,
            ZlMediaKitHookEvent::OnRecordTs { media_server_tag, stream, app, schema, file_path, file_size, segment_index, segment_duration, .. } => 
                self.on_record_ts(&media_server_tag, &stream, &app, schema.as_deref(), file_path.as_ref(), file_size, segment_index, segment_duration).await,
            ZlMediaKitHookEvent::OnFlowReport { media_server_tag, stream, ip, duration, total_bytes, player, .. } => 
                self.on_flow_report(&media_server_tag, &stream, ip.as_ref(), duration, total_bytes, player).await,
            ZlMediaKitHookEvent::OnHttpAccess { media_server_tag, ip, path, .. } => 
                self.on_http_access(&media_server_tag, ip.as_ref(), path.as_ref()).await,
            ZlMediaKitHookEvent::OnRtspRealm { media_server_tag, stream, ip, params, .. } => 
                self.on_rtsp_realm(&media_server_tag, &stream, ip.as_ref(), params.as_ref()).await,
            ZlMediaKitHookEvent::OnRtspAuth { media_server_tag, stream, ip, params, user_name, .. } => 
                self.on_rtsp_auth(&media_server_tag, &stream, ip.as_ref(), user_name.as_deref(), params.as_ref()).await,
            ZlMediaKitHookEvent::OnShellLogin { media_server_tag, ip, user_name, .. } => 
                self.on_shell_login(&media_server_tag, ip.as_ref(), user_name.as_deref()).await,
            ZlMediaKitHookEvent::OnStreamChanged { media_server_tag, app, stream, regist, .. } => 
                self.on_stream_changed(&media_server_tag, &app, &stream, regist).await,
            ZlMediaKitHookEvent::OnStreamNoneReader { media_server_tag, app, stream, .. } => 
                self.on_stream_none_reader(&media_server_tag, &app, &stream).await,
            ZlMediaKitHookEvent::OnStreamNotFound { media_server_tag, stream, ip, .. } => 
                self.on_stream_not_found(&media_server_tag, &stream, ip.as_ref()).await,
            ZlMediaKitHookEvent::OnServerStarted { media_server_tag, .. } => 
                self.on_server_started(&media_server_tag).await,
            ZlMediaKitHookEvent::OnServerKeepalive { media_server_tag, data } => 
                self.on_server_keepalive(&media_server_tag, data.as_ref()).await,
            ZlMediaKitHookEvent::OnServerExited { media_server_tag, .. } => 
                self.on_server_exited(&media_server_tag).await,
            ZlMediaKitHookEvent::OnSendRtpStopped { media_server_tag, stream_id, local_port, ssrc, ip, rtp } => 
                self.on_send_rtp_stopped(&media_server_tag, stream_id.as_deref(), local_port, ssrc.as_deref(), ip.as_ref(), rtp).await,
            ZlMediaKitHookEvent::OnRtpServerTimeout { media_server_tag, stream_id, .. } => 
                self.on_rtp_server_timeout(&media_server_tag, stream_id.as_deref()).await,
        }
    }

    async fn on_play(&self, media_server_tag: &str, stream: &str, ip: Option<&String>, params: Option<&String>, _app: Option<&String>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_play: server={}, stream={}, ip={:?}, params={:?}", 
            media_server_tag, stream, ip, params);
        HookResponse::allow()
    }

    async fn on_publish(&self, media_server_tag: &str, stream: &str, ip: Option<&String>, _app: Option<&String>, params: Option<&String>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_publish: server={}, stream={}, ip={:?}, params={:?}", 
            media_server_tag, stream, ip, params);

        // 验证 secret
        if self.validate_secret(params.map(|s| s.as_str())) {
            tracing::debug!("[ZlMediaKit Hook] on_publish: admin access, skipping auth");
            return HookResponse::allow();
        }

        // 验证 media_server_tag
        if self.validate_media_server(media_server_tag).is_none() {
            tracing::warn!("[ZlMediaKit Hook] on_publish: unknown media_server_tag={}", media_server_tag);
            return HookResponse::deny("Unknown media server");
        }

        // 注册推流
        if let Err(e) = self.register_push_stream(media_server_tag, stream).await {
            tracing::warn!("[ZlMediaKit Hook] on_publish: failed to register: {}", e);
            return HookResponse::deny("Failed to register stream");
        }

        HookResponse::allow()
    }

    async fn on_stop(&self, media_server_tag: &str, stream: &str, _ip: Option<&String>, _params: Option<&String>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_stop: server={}, stream={}", media_server_tag, stream);

        let sessions = self.find_sessions_by_stream(stream).await;
        for session in sessions {
            let mut s = session;
            s.terminate();
            self.repo.sessions_cache().insert(s.id, s);
        }

        if let Err(e) = self.decrement_stream_viewers(stream).await {
            tracing::warn!("[ZlMediaKit Hook] on_stop: failed to decrement viewers: {}", e);
        }

        HookResponse::allow()
    }

    async fn on_keepalive(&self, media_server_tag: &str, stream_id: Option<&String>, _ip: Option<&String>) -> HookResponse {
        tracing::trace!("[ZlMediaKit Hook] on_keepalive: server={}, stream={:?}", media_server_tag, stream_id);

        let Some(stream_id) = stream_id else {
            return HookResponse::allow();
        };

        let sessions = self.find_sessions_by_stream(stream_id).await;
        for session in sessions {
            let mut s = session;
            s.touch();
            s.refresh_expiry(self.session_expiry_secs);
            self.repo.sessions_cache().insert(s.id, s);
        }

        if let Err(e) = self.touch_stream_keepalive(stream_id).await {
            tracing::trace!("[ZlMediaKit Hook] on_keepalive: failed to update stream keepalive: {}", e);
        }

        const STREAM_TTL_SECS: u64 = 86400;
        if let Err(e) = self.redis.expire_key(&format!("stream:{}", stream_id), STREAM_TTL_SECS).await {
            tracing::trace!("[ZlMediaKit Hook] on_keepalive: failed to refresh stream TTL: {}", e);
        }

        HookResponse::allow()
    }

    async fn on_rtcp_stats(&self, media_server_tag: &str, stream: &str, bytes_sent: Option<u64>, bytes_received: Option<u64>) -> HookResponse {
        tracing::trace!("[ZlMediaKit Hook] on_rtcp_stats: server={}, stream={}, bytes_sent={:?}, bytes_recv={:?}", 
            media_server_tag, stream, bytes_sent, bytes_received);

        let sessions = self.find_sessions_by_stream(stream).await;
        for session in sessions {
            let mut s = session;
            s.update_stats(bytes_sent.unwrap_or(0), bytes_received.unwrap_or(0));
            self.repo.sessions_cache().insert(s.id, s);
        }

        if let Err(e) = self.update_stream_bandwidth(stream, bytes_received.unwrap_or(0), bytes_sent.unwrap_or(0)).await {
            tracing::trace!("[ZlMediaKit Hook] on_rtcp_stats: failed to update bandwidth: {}", e);
        }

        HookResponse::allow()
    }

    async fn on_record_done(&self, media_server_tag: &str, stream: &str, _app: &str, file_path: Option<&String>, file_size: Option<u64>, time_len: Option<f64>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_record_done: server={}, stream={}, file={:?}, size={:?}, duration={:?}", 
            media_server_tag, stream, file_path, file_size, time_len);

        let filename = file_path
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .map(String::from)
            .unwrap_or_else(|| stream.to_string());

        let size = file_size.unwrap_or(0);
        let duration = time_len.map(|t| t as u64).unwrap_or(0);

        if let Err(e) = self.recording_service.finalize_recording(stream, filename, size, duration).await {
            tracing::error!("[ZlMediaKit Hook] on_record_done: failed to finalize recording: {}", e);
        }

        HookResponse::allow()
    }

    async fn on_record_ts(&self, media_server_tag: &str, stream: &str, app: &str, schema: Option<&str>, file_path: Option<&String>, file_size: Option<u64>, segment_index: Option<u32>, segment_duration: Option<f64>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_record_ts: server={}, stream={}, app={}, schema={:?}, file={:?}, size={:?}, segment={:?}, duration={:?}", 
            media_server_tag, stream, app, schema, file_path, file_size, segment_index, segment_duration);

        HookResponse::allow()
    }

    async fn on_flow_report(&self, media_server_tag: &str, stream: &str, ip: Option<&String>, duration: Option<u32>, total_bytes: Option<u64>, player: Option<bool>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_flow_report: server={}, stream={}, player={:?}, bytes={:?}, duration={:?}", 
            media_server_tag, stream, player, total_bytes, duration);
        HookResponse::allow()
    }

    async fn on_http_access(&self, media_server_tag: &str, ip: Option<&String>, path: Option<&String>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_http_access: server={}, path={:?}, ip={:?}", media_server_tag, path, ip);
        let allowed_path = path.map(|s| s.as_str()).unwrap_or("");
        HookResponse::http_access(allowed_path, 600)
    }

    async fn on_rtsp_realm(&self, media_server_tag: &str, stream: &str, ip: Option<&String>, params: Option<&String>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_rtsp_realm: server={}, stream={}, ip={:?}", media_server_tag, stream, ip);

        // 如果启用了 secret 验证且参数匹配，直接返回空 realm（不需要鉴权）
        if self.validate_secret(params.map(|s| s.as_str())) {
            return HookResponse::rtsp_realm("");
        }

        // 查找设备进行 RTSP 认证
        if let Some(device) = self.find_device_by_stream_key(stream) {
            // 返回播放用户名作为 realm，用于 ZLMediaKit 挑战客户端
            let realm = device.playback_username.as_deref().unwrap_or("RustCam");
            tracing::debug!("[ZlMediaKit Hook] on_rtsp_realm: using realm={} for stream={}", realm, stream);
            return HookResponse::rtsp_realm(realm);
        }

        tracing::warn!("[ZlMediaKit Hook] on_rtsp_realm: device not found for stream={}", stream);
        HookResponse::rtsp_realm("")
    }

    async fn on_rtsp_auth(&self, media_server_tag: &str, stream: &str, ip: Option<&String>, user_name: Option<&str>, params: Option<&String>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_rtsp_auth: server={}, stream={}, user={:?}", media_server_tag, stream, user_name);

        // 验证 secret
        if self.validate_secret(params.map(|s| s.as_str())) {
            return HookResponse::rtsp_auth(false, "");
        }

        // 查找设备进行密码验证
        let Some(device) = self.find_device_by_stream_key(stream) else {
            tracing::warn!("[ZlMediaKit Hook] on_rtsp_auth: device not found for stream={}", stream);
            return HookResponse::rtsp_auth(false, "Device not found");
        };

        // 验证用户名匹配
        let expected_user = device.playback_username.as_deref().unwrap_or("");
        let provided_user = user_name.unwrap_or("");
        if !expected_user.is_empty() && provided_user != expected_user {
            tracing::warn!("[ZlMediaKit Hook] on_rtsp_auth: username mismatch for stream={}, expected={}, got={}", 
                stream, expected_user, provided_user);
            return HookResponse::rtsp_auth(false, "Invalid username");
        }

        // 验证密码
        let expected_pass = device.playback_password.as_deref().unwrap_or("");
        if !expected_pass.is_empty() {
            tracing::debug!("[ZlMediaKit Hook] on_rtsp_auth: requiring password verification for stream={}", stream);
            // 返回密码，ZLMediaKit 会进行验证
            return HookResponse::rtsp_auth(false, expected_pass);
        }

        // 设备没有设置密码，允许访问
        tracing::debug!("[ZlMediaKit Hook] on_rtsp_auth: no password configured, allowing access for stream={}", stream);
        HookResponse::rtsp_auth(false, "")
    }

    async fn on_shell_login(&self, media_server_tag: &str, ip: Option<&String>, user_name: Option<&str>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_shell_login: server={}, user={:?}", media_server_tag, user_name);
        HookResponse::allow()
    }

    async fn on_stream_changed(&self, media_server_tag: &str, app: &str, stream: &str, regist: bool) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_stream_changed: server={}, app={}, stream={}, regist={}", 
            media_server_tag, app, stream, regist);

        if !regist {
            if let Some(s) = self.find_stream_by_stream_key(stream) {
                let mut s = s;
                s.stopped();
                if let Err(e) = self.repo.update_stream(&s).await {
                    tracing::warn!("[ZlMediaKit Hook] on_stream_changed: failed to stop stream: {}", e);
                }
            }
            return HookResponse::allow();
        }

        if let Some(s) = self.find_stream_by_stream_key(stream) {
            let mut s = s;
            s.update_keepalive();
            if s.state != crate::domain::StreamState::Active {
                s.start();
            }
            if let Err(e) = self.repo.update_stream(&s).await {
                tracing::warn!("[ZlMediaKit Hook] on_stream_changed: failed to update stream: {}", e);
            }
        }

        HookResponse::allow()
    }

    async fn on_stream_none_reader(&self, media_server_tag: &str, app: &str, stream: &str) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_stream_none_reader: server={}, app={}, stream={}", 
            media_server_tag, app, stream);
        HookResponse::allow()
    }

    async fn on_stream_not_found(&self, media_server_tag: &str, stream: &str, ip: Option<&String>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_stream_not_found: server={}, stream={}, ip={:?}", 
            media_server_tag, stream, ip);
        HookResponse::allow()
    }

    async fn on_server_started(&self, media_server_tag: &str) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_server_started: server={}, restarting streams directly", media_server_tag);

        if let Some(ref svc) = self.stream_recovery_service {
            svc.restart_streams_for_media_server(media_server_tag).await;
        } else {
            tracing::warn!("[ZlMediaKit Hook] on_server_started: stream_recovery_service not available");
        }

        HookResponse::allow()
    }

    async fn on_server_keepalive(&self, media_server_tag: &str, data: Option<&ZlMediaKitServerKeepaliveData>) -> HookResponse {
        tracing::trace!("[ZlMediaKit Hook] on_server_keepalive: server={}, data={:?}", media_server_tag, data);
        HookResponse::allow()
    }

    async fn on_server_exited(&self, media_server_tag: &str) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_server_exited: server={}", media_server_tag);

        if let Some(ref svc) = self.stream_recovery_service {
            svc.mark_streams_recovering_for_media_server(media_server_tag).await;
        }

        HookResponse::allow()
    }

    async fn on_send_rtp_stopped(&self, media_server_tag: &str, stream_id: Option<&str>, local_port: Option<u16>, ssrc: Option<&str>, ip: Option<&String>, rtp: Option<bool>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_send_rtp_stopped: server={}, stream_id={:?}, port={:?}, ssrc={:?}, ip={:?}, rtp={:?}", 
            media_server_tag, stream_id, local_port, ssrc, ip, rtp);
        HookResponse::allow()
    }

    async fn on_rtp_server_timeout(&self, media_server_tag: &str, stream_id: Option<&str>) -> HookResponse {
        tracing::info!("[ZlMediaKit Hook] on_rtp_server_timeout: server={}, stream_id={:?}", 
            media_server_tag, stream_id);
        HookResponse::allow()
    }

    // ============== 辅助方法 ==============

    fn find_stream_by_stream_key(&self, stream_key: &str) -> Option<crate::domain::Stream> {
        let cache_key = stream_key.replace('_', "/");
        self.repo.streams_cache()
            .get(&cache_key)
            .map(|s| s.clone())
    }

    fn find_device_by_stream_key(&self, stream_key: &str) -> Option<crate::domain::Device> {
        let tag = self.find_stream_by_stream_key(stream_key)?.device_tag?;
        self.repo.devices_cache().get(&tag).map(|d| d.clone())
    }

    async fn increment_stream_viewers(&self, stream_key: &str) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.increment_viewers();
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    async fn decrement_stream_viewers(&self, stream_key: &str) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.decrement_viewers();
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    async fn touch_stream_keepalive(&self, stream_key: &str) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.update_keepalive();
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    async fn update_stream_bandwidth(&self, stream_key: &str, bytes_in: u64, bytes_out: u64) -> Result<()> {
        if let Some(s) = self.find_stream_by_stream_key(stream_key) {
            let mut s = s;
            s.update_bandwidth(bytes_in, bytes_out);
            self.repo.update_stream(&s).await?;
        }
        Ok(())
    }

    async fn register_push_stream(&self, media_server_tag: &str, stream_id: &str) -> Result<()> {
        let device_id = self.find_device_by_push_stream(stream_id);

        if let Some(s) = self.find_stream_by_stream_key(stream_id) {
            let mut s = s;
            s.update_keepalive();
            if s.state == crate::domain::StreamState::Idle || s.state == crate::domain::StreamState::Error {
                s.start();
            }
            self.repo.update_stream(&s).await?;
            tracing::info!("[ZlMediaKit Hook] on_publish: updated push stream {} (device_tag={})", stream_id, s.device_tag.as_deref().unwrap_or("unknown"));
        } else {
            let (device_tag, channel_tag) = crate::domain::parse_stream_key(stream_id)
                .unwrap_or((stream_id.to_string(), stream_id.to_string()));
            let app = device_id
                .as_ref()
                .and_then(|id| self.repo.devices_cache().get(id))
                .map(|d| d.app.clone().unwrap_or_else(|| "live".to_string()))
                .unwrap_or_else(|| "live".to_string());
            let mut stream = crate::domain::Stream::new(
                media_server_tag.to_string(),
                device_tag,
                channel_tag,
                app,
                uuid::Uuid::new_v4().to_string(),
            );
            stream.start();
            self.repo.create_stream(&stream).await?;
            tracing::info!("[ZlMediaKit Hook] on_publish: registered push stream {} on server {}", stream_id, media_server_tag);
        }

        if let Some(ref did) = device_id {
            if let Some(device) = self.repo.devices_cache().get(did) {
                let mut device = (*device).clone();
                device.set_online();
                self.repo.update_device(&device).await?;
                tracing::info!("[ZlMediaKit Hook] on_publish: device {} is now online", did);
            }
        }

        Ok(())
    }

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

    async fn find_sessions_by_stream(&self, stream_id: &str) -> Vec<crate::domain::Session> {
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
}

// Alias for backwards compatibility
#[allow(unused)]
pub type HookService = ZlMediaKitHookHandler;
