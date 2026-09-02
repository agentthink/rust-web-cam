use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::device::PlayLinks;
use crate::config::MediaServerConfig;

pub fn format_rtsp_auth(auth: Option<(&str, &str)>) -> String {
    match auth {
        Some((u, p)) => format!("{}:{}@", u, p),
        None => String::new(),
    }
}

pub fn host_with_port_from_config(config: &MediaServerConfig, protocol: Protocol) -> String {
    let (host, url_port) = extract_host_and_port(&config.url);
    tracing::debug!("[host_with_port] config.url={}, extracted host={}, url_port={:?}, protocol={:?}", config.url, host, url_port, protocol);
    
    let port = match protocol {
        Protocol::Rtsp => config.protocol_ports.rtsp.unwrap_or(554),
        Protocol::Rtmp => config.protocol_ports.rtmp.unwrap_or(1935),
        Protocol::Hls => config.protocol_ports.hls.unwrap_or(8080),
        Protocol::Http => config.protocol_ports.http.unwrap_or(8080),
        Protocol::Flv => config.protocol_ports.http_flv.unwrap_or(8080),
        Protocol::WsFlv => config.protocol_ports.ws_flv.unwrap_or(8080),
        Protocol::WebRTC => config.protocol_ports.webrtc.unwrap_or(8080),
    };
    format!("{}:{}", host, port)
}

pub fn extract_host_and_port(url: &str) -> (String, Option<u16>) {
    let url = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    
    let (host_port, _path) = if let Some(idx) = url.find('/') {
        (&url[..idx], &url[idx..])
    } else {
        (url, "")
    };
    
    let dot_count = host_port.matches('.').count();
    let is_ipv4 = dot_count == 3 && host_port.chars().filter(|c| *c == ':').count() == 1;
    
    if is_ipv4 {
        if let Some(idx) = host_port.rfind(':') {
            let host = &host_port[..idx];
            let port_str = &host_port[idx+1..];
            let port = port_str.parse::<u16>().ok();
            return (host.to_string(), port);
        }
    }
    
    if let Some(idx) = host_port.find(':') {
        let host = &host_port[..idx];
        let port_str = &host_port[idx+1..];
        let port = port_str.parse::<u16>().ok();
        return (host.to_string(), port);
    }
    
    (host_port.to_string(), None)
}

pub fn base_host_from_config(config: &MediaServerConfig) -> String {
    let (host, _) = extract_host_and_port(&config.url);
    host
}

#[derive(Debug, Clone, Copy, Default)]
pub enum RtpTransport {
    #[default]
    Udp,
    TcpPassive,
    TcpActive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub client_ip: String,
    pub stream: String,
    pub app: String,
    pub protocol: String,
    pub connected_at: i64,
    pub alive_seconds: u64,
}

pub mod zlmediakit;
pub mod srs;
pub mod xiu;
mod client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub name: String,
    pub server_type: String,
    pub online: bool,
    pub session_count: u32,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub bandwidth_in: u64,
    pub bandwidth_out: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<i64>,
}

impl fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}] - sessions: {}, CPU: {:.1}%, Mem: {:.1}%",
               self.name, self.server_type, self.session_count, self.cpu_usage, self.memory_usage)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub stream_key: String,
    pub play_url: String,
    pub rtsp_url: String,
    pub rtmp_url: String,
    pub hls_url: String,
    pub webrtc_url: String,
    #[serde(default)]
    pub flv_url: Option<String>,
    #[serde(default)]
    pub web_flv_url: Option<String>,
    pub media_server_id: String,
    pub media_server_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Rtsp,
    Rtmp,
    Hls,
    Http,
    WebRTC,
    Flv,
    WsFlv,
}

impl Default for Protocol {
    fn default() -> Self { Protocol::Hls }
}

#[async_trait]
pub trait MediaServerAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn tag(&self) -> &str;
    fn server_type(&self) -> &str;
    async fn is_online(&self) -> bool;
    async fn get_status(&self) -> anyhow::Result<ServerStatus>;
    async fn add_stream_proxy(&self, app: &str, stream_key: &str, rtsp_url: &str) -> anyhow::Result<StreamInfo>;
    async fn remove_stream_proxy(&self, app: &str, stream_key: &str) -> anyhow::Result<()>;
    async fn get_play_url(&self, app: &str, stream_key: &str, protocol: Protocol) -> anyhow::Result<String>;
    async fn get_session_count(&self) -> anyhow::Result<u32>;
    async fn get_sessions(&self) -> anyhow::Result<Vec<serde_json::Value>>;
    async fn is_stream_online(&self, app: &str, stream_key: &str) -> anyhow::Result<bool>;
    async fn ptz_control(&self, stream_key: &str, command: &str, channel: u8) -> anyhow::Result<()>;
    async fn start_recording(&self, app: &str, stream_key: &str, format: &str, output_path: Option<&str>) -> anyhow::Result<RecordingInfo>;
    async fn stop_recording(&self, app: &str, stream_key: &str, format: &str) -> anyhow::Result<()>;
    async fn is_recording(&self, stream_key: &str, format: &str) -> anyhow::Result<bool>;
    async fn list_recordings(&self, app: &str, stream_key: &str) -> anyhow::Result<Vec<RecordingFile>>;
    async fn open_rtp_server(&self, stream_id: &str, port: u16, transport: RtpTransport) -> anyhow::Result<(u16, String)>;
    async fn close_rtp_server(&self, stream_id: &str) -> anyhow::Result<()>;
    async fn get_media_info(&self, app: &str, stream_key: &str) -> anyhow::Result<Option<serde_json::Value>>;
    async fn build_play_links(
        &self,
        app: &str,
        stream_key: &str,
        token: &str,
        expires_at: i64,
        rtsp_auth: Option<(&str, &str)>,
    ) -> anyhow::Result<PlayLinks>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub stream_key: String,
    pub output_path: String,
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingFile {
    pub filename: String,
    pub path: String,
    pub size: u64,
    pub duration_secs: u64,
    pub created_at: i64,
    pub stream_key: Option<String>,
    pub media_server_name: Option<String>,
}