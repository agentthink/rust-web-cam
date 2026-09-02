use async_trait::async_trait;
use serde::Deserialize;
use crate::adapter::media_server::{MediaServerAdapter, Protocol, RecordingInfo, RecordingFile, RtpTransport, ServerStatus, StreamInfo, host_with_port_from_config, format_rtsp_auth};
use crate::config::MediaServerConfig;
use crate::domain::device::PlayLinks;

pub struct XiuAdapter {
    config: MediaServerConfig,
    base_url: String,
}

impl XiuAdapter {
    pub fn new(config: MediaServerConfig) -> Self {
        let base_url = config.url.trim_end_matches('/').to_string();
        Self { config, base_url }
    }

    fn host_with_port(&self, protocol: Protocol) -> String {
        host_with_port_from_config(&self.config, protocol)
    }
}

#[async_trait]
impl MediaServerAdapter for XiuAdapter {
    fn name(&self) -> &str { &self.config.name }
    fn tag(&self) -> &str { &self.config.server_tag }
    fn server_type(&self) -> &str { "xiu" }

    async fn is_online(&self) -> bool {
        let url = format!("{}/api/status", self.base_url);
        reqwest::Client::new().get(&url).header("Authorization", format!("Bearer {}", self.config.api_key))
            .send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }

    async fn get_status(&self) -> anyhow::Result<ServerStatus> {
        Ok(ServerStatus {
            name: self.config.name.clone(), server_type: "xiu".to_string(), online: true,
            session_count: 0, cpu_usage: 0.0, memory_usage: 0.0,
            bandwidth_in: 0, bandwidth_out: 0, last_heartbeat: None,
        })
    }

    async fn add_stream_proxy(&self, _app: &str, stream_key: &str, _rtsp_url: &str) -> anyhow::Result<StreamInfo> {
        let rtsp_host = self.host_with_port(Protocol::Rtsp);
        let rtmp_host = self.host_with_port(Protocol::Rtmp);
        let http_host = self.host_with_port(Protocol::Hls);
        Ok(StreamInfo {
            stream_key: stream_key.to_string(),
            play_url: format!("http://{}/{}", http_host, stream_key),
            rtsp_url: format!("rtsp://{}/{}", rtsp_host, stream_key),
            rtmp_url: format!("rtmp://{}/live/{}", rtmp_host, stream_key),
            hls_url: format!("http://{}/live/{}.m3u8", http_host, stream_key),
            webrtc_url: String::new(), flv_url: None, web_flv_url: None,
            media_server_id: self.config.name.clone(), media_server_name: self.config.name.clone(),
        })
    }

    async fn remove_stream_proxy(&self, _app: &str, _stream_key: &str) -> anyhow::Result<()> { Ok(()) }
    async fn get_play_url(&self, _app: &str, stream_key: &str, protocol: Protocol) -> anyhow::Result<String> {
        let host = self.host_with_port(protocol);
        match protocol {
            Protocol::Rtsp => Ok(format!("rtsp://{}/{}", host, stream_key)),
            Protocol::Rtmp => Ok(format!("rtmp://{}/live/{}", host, stream_key)),
            Protocol::Hls => Ok(format!("http://{}/live/{}.m3u8", host, stream_key)),
            _ => Ok(String::new()),
        }
    }
    async fn get_session_count(&self) -> anyhow::Result<u32> { Ok(0) }
    async fn get_sessions(&self) -> anyhow::Result<Vec<serde_json::Value>> { Ok(vec![]) }
    async fn is_stream_online(&self, _app: &str, _stream_key: &str) -> anyhow::Result<bool> { Ok(false) }
    async fn ptz_control(&self, _stream_key: &str, _command: &str, _channel: u8) -> anyhow::Result<()> { Ok(()) }
    async fn start_recording(&self, _app: &str, _stream_key: &str, _format: &str, _output_path: Option<&str>) -> anyhow::Result<RecordingInfo> {
        Ok(RecordingInfo { stream_key: String::new(), output_path: String::new(), started_at: chrono::Utc::now().timestamp() })
    }
    async fn stop_recording(&self, _app: &str, _stream_key: &str, _format: &str) -> anyhow::Result<()> { Ok(()) }
    async fn is_recording(&self, _stream_key: &str, _format: &str) -> anyhow::Result<bool> { Ok(false) }
    async fn list_recordings(&self, _app: &str, _stream_key: &str) -> anyhow::Result<Vec<RecordingFile>> { Ok(vec![]) }
    async fn open_rtp_server(&self, _stream_id: &str, _port: u16, _transport: RtpTransport) -> anyhow::Result<(u16, String)> {
        Err(anyhow::anyhow!("RTP server not supported for Xiu"))
    }
    async fn close_rtp_server(&self, _stream_id: &str) -> anyhow::Result<()> { Ok(()) }
    async fn get_media_info(&self, _app: &str, _stream_key: &str) -> anyhow::Result<Option<serde_json::Value>> { Ok(None) }

    async fn build_play_links(
        &self,
        _app: &str,
        stream_key: &str,
        token: &str,
        expires_at: i64,
        rtsp_auth: Option<(&str, &str)>,
    ) -> anyhow::Result<PlayLinks> {
        let rtsp_host = self.host_with_port(Protocol::Rtsp);
        let rtmp_host = self.host_with_port(Protocol::Rtmp);
        let http_host = self.host_with_port(Protocol::Hls);

        let rtsp_auth_str = format_rtsp_auth(rtsp_auth);

        Ok(PlayLinks {
            token: token.to_string(),
            stream_id: stream_key.to_string(),
            expires_at,
            ports: self.config.protocol_ports.clone(),
            rtsp_signaling: Some(format!(
                "rtsp://{}{}/{}?token={}",
                rtsp_auth_str, rtsp_host, stream_key, token
            )),
            rtsp_media: Some(format!(
                "rtsp://{}{}/{}?token={}",
                rtsp_auth_str, rtsp_host, stream_key, token
            )),
            flv: Some(format!(
                "http://{}/live/{}.flv?token={}",
                http_host, stream_key, token
            )),
            hls: Some(format!(
                "http://{}/live/{}.m3u8?token={}",
                http_host, stream_key, token
            )),
            webrtc: None,
            web_flv: Some(format!(
                "ws://{}/live/{}.flv?token={}",
                http_host, stream_key, token
            )),
        })
    }
}