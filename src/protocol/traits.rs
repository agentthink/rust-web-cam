use std::sync::Arc;
use crate::domain::traits::{DeviceLookup, StreamManager, EventPublisher, CacheStore};
use crate::infrastructure::cluster::ClusterManager;
use crate::protocol::rtsp::rtp_tunnel::RtpTunnel;
use crate::protocol::adapter_manager::AdapterEntry;
use crate::config::AppConfig;

/// 协议适配器依赖
#[derive(Clone)]
pub struct ProtocolDeps {
    pub device_lookup: Arc<dyn DeviceLookup>,
    pub stream_manager: Arc<dyn StreamManager>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub cluster: Arc<ClusterManager>,
    pub rtp_tunnel: Arc<RtpTunnel>,
    pub redis: Arc<dyn CacheStore>,
    pub config: Arc<AppConfig>,
    pub register_fn: Arc<dyn Fn(String, AdapterEntry) + Send + Sync>,
    pub unregister_fn: Arc<dyn Fn(String) + Send + Sync>,
}

impl ProtocolDeps {
    pub fn new(
        device_lookup: Arc<dyn DeviceLookup>,
        stream_manager: Arc<dyn StreamManager>,
        event_publisher: Arc<dyn EventPublisher>,
        cluster: Arc<ClusterManager>,
        rtp_tunnel: Arc<RtpTunnel>,
        redis: Arc<dyn CacheStore>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            device_lookup,
            stream_manager,
            event_publisher,
            cluster,
            rtp_tunnel,
            redis,
            config,
            register_fn: Arc::new(|_, _| {}),
            unregister_fn: Arc::new(|_| {}),
        }
    }

    pub fn with_registration(
        self,
        register_fn: Arc<dyn Fn(String, AdapterEntry) + Send + Sync>,
        unregister_fn: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Self {
        Self {
            register_fn,
            unregister_fn,
            ..self
        }
    }
}

impl std::fmt::Debug for ProtocolDeps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtocolDeps")
            .field("cluster_servers", &self.cluster.server_count())
            .field("rtp_tunnels", &self.rtp_tunnel.tunnel_count())
            .field("rtsp_auth", &self.config.rtsp_auth.enabled)
            .finish()
    }
}

// ═══════════════════════════════════════════════════════════════
// 测试支持 (仅在 cfg(test) 下编译)
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
impl ProtocolDeps {
    pub fn for_test() -> Self {
        use crate::error::AppError;
        use crate::adapter::media_server::StreamInfo;

        struct TestDeviceLookup;
        #[async_trait::async_trait]
        impl DeviceLookup for TestDeviceLookup {
            async fn find_by_tag(&self, tag: &str) -> Option<crate::domain::Device> {
                let mut device = crate::domain::Device::new(
                    format!("Test-{}", tag),
                    crate::domain::Protocol::Rtsp,
                );
                device.device_tag = Some(tag.to_string());
                device.device_password = Some("test_password".to_string());
                Some(device)
            }
            async fn find_by_stream_key(&self, _: &str) -> Option<crate::domain::Device> { None }
            async fn find_by_protocol_and_host(&self, _: &crate::domain::Protocol, _: &str) -> Option<crate::domain::Device> { None }
            async fn get_device(&self, _: i64) -> Result<Option<crate::domain::Device>, AppError> { Ok(None) }
            async fn set_online(&self, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn set_offline(&self, _: &str, _: Option<&str>) -> Result<(), AppError> { Ok(()) }
            async fn log_ptz_control(&self, _: Option<uuid::Uuid>, _: i64, _: &str, _: u8, _: bool, _: Option<String>, _: Option<String>) -> Result<(), AppError> { Ok(()) }
            async fn log_ptz_result(&self, _: i64, _: Option<&str>, _: Option<u16>, _: &str, _: Option<String>) -> Result<(), AppError> { Ok(()) }
            fn broadcast_ptz_result(&self, _: i64, _: &str, _: &str, _: &str, _: Option<u16>, _: Option<&str>) {}
            fn get_stats(&self) -> serde_json::Value { serde_json::json!({}) }
            async fn list_online_devices(&self) -> Vec<crate::domain::Device> { vec![] }
        }

        struct TestStreamManager;
        #[async_trait::async_trait]
        impl StreamManager for TestStreamManager {
            async fn start_pull_stream(&self, _: &str, _: &str, _: &str) -> Result<StreamInfo, AppError> {
                Ok(StreamInfo {
                    stream_key: "test_stream".to_string(),
                    play_url: "http://localhost/test".to_string(),
                    rtsp_url: "rtsp://localhost/test".to_string(),
                    rtmp_url: String::new(),
                    hls_url: String::new(),
                    webrtc_url: String::new(),
                    flv_url: None,
                    web_flv_url: None,
                    media_server_id: "test".to_string(),
                    media_server_name: "test".to_string(),
                })
            }
            async fn start_gb28181_stream(&self, _: &str, _: &str, _: &str) -> Result<StreamInfo, AppError> {
                Err(AppError::Internal("not implemented".into()))
            }
            async fn stop_stream(&self, _: &str, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn stop_streams_by_device(&self, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn stop_streams_by_channel(&self, _: &str, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn generate_token(&self, _: &str) -> Result<String, AppError> { Ok("test_token".into()) }
            async fn validate_token(&self, _: &str) -> Option<String> { Some("test_device".into()) }
            async fn build_play_links(&self, device: &crate::domain::Device, token: &str, stream_id: &str) -> crate::domain::device::PlayLinks {
                crate::domain::device::PlayLinks {
                    token: token.to_string(), stream_id: stream_id.to_string(), expires_at: 0,
                    ports: Default::default(),
                    rtsp_signaling: None, rtsp_media: None, flv: None, hls: None, webrtc: None, web_flv: None,
                }
            }
            async fn get_stats(&self) -> serde_json::Value { serde_json::json!({}) }
            async fn get_stream_by_stream_key(&self, _: &str, _: &str) -> Option<crate::domain::Stream> { None }
            async fn update_stream_state(&self, _: &crate::domain::Stream) -> Result<(), AppError> { Ok(()) }
        }

        struct TestEventPublisher;
        #[async_trait::async_trait]
        impl EventPublisher for TestEventPublisher {
            async fn publish(&self, _: crate::protocol::event::SignalEvent) -> Result<(), AppError> { Ok(()) }
            fn subscribe(&self) -> tokio::sync::broadcast::Receiver<crate::protocol::event::SignalEvent> {
                let (_, rx) = tokio::sync::broadcast::channel(1);
                rx
            }
        }

        // ✅ 使用修复后的 CacheStore trait 方法（非泛型）
        struct TestCacheStore;
        #[async_trait::async_trait]
        impl CacheStore for TestCacheStore {
            async fn set_json(&self, _: &str, _: &serde_json::Value, _: Option<u64>) -> Result<(), AppError> { Ok(()) }
            async fn get_json(&self, _: &str) -> Result<Option<serde_json::Value>, AppError> { Ok(None) }
            async fn del(&self, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn set_stream_info(&self, _: &str, _: &StreamInfo) -> Result<(), AppError> { Ok(()) }
            async fn get_stream_info(&self, _: &str) -> Result<Option<StreamInfo>, AppError> { Ok(None) }
            async fn delete_stream_info(&self, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn publish_event_json(&self, _: &str, _: &serde_json::Value) -> Result<(), AppError> { Ok(()) }
            async fn incr(&self, _: &str) -> Result<i64, AppError> { Ok(0) }
            async fn exists(&self, _: &str) -> Result<bool, AppError> { Ok(false) }
            async fn expire(&self, _: &str, _: u64) -> Result<(), AppError> { Ok(()) }
            async fn sadd(&self, _: &str, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn srem(&self, _: &str, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn smembers(&self, _: &str) -> Result<Vec<String>, AppError> { Ok(vec![]) }
            async fn hset(&self, _: &str, _: &str, _: &str) -> Result<(), AppError> { Ok(()) }
            async fn hmset(&self, _: &str, _: &[(&str, &str)]) -> Result<(), AppError> { Ok(()) }
            async fn hget(&self, _: &str, _: &str) -> Result<Option<String>, AppError> { Ok(None) }
            async fn hgetall(&self, _: &str) -> Result<std::collections::HashMap<String, String>, AppError> { Ok(std::collections::HashMap::new()) }
        }

        Self {
            device_lookup: Arc::new(TestDeviceLookup),
            stream_manager: Arc::new(TestStreamManager),
            event_publisher: Arc::new(TestEventPublisher),
            cluster: Arc::new(ClusterManager::new_from_configs(vec![])),
            rtp_tunnel: Arc::new(RtpTunnel::new()),
            redis: Arc::new(TestCacheStore),
            config: Arc::new(AppConfig::default()),
            register_fn: Arc::new(|_, _| {}),
            unregister_fn: Arc::new(|_| {}),
        }
    }
}