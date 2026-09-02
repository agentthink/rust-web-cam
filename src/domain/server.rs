use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Zlmediakit,
    Srs,
    Xiu,
}

impl ServerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerType::Zlmediakit => "zlmediakit",
            ServerType::Srs => "srs",
            ServerType::Xiu => "xiu",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "zlmediakit" => Some(ServerType::Zlmediakit),
            "srs" => Some(ServerType::Srs),
            "xiu" => Some(ServerType::Xiu),
            _ => None,
        }
    }
}

impl std::fmt::Display for ServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerProtocolPorts {
    #[serde(default)]
    pub rtsp: Option<u16>,
    #[serde(default)]
    pub rtmp: Option<u16>,
    #[serde(default)]
    pub hls: Option<u16>,
    #[serde(default)]
    pub http: Option<u16>,
    #[serde(default)]
    pub https: Option<u16>,
    #[serde(default)]
    pub webrtc: Option<u16>,
    #[serde(default)]
    pub rtp_tcp: Option<u16>,
    #[serde(default)]
    pub http_flv: Option<u16>,
    #[serde(default)]
    pub ws_flv: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub server_type: ServerType,
    pub weight: u32,
    pub enabled: bool,
    pub server_tag: String,
    pub protocol_ports: ServerProtocolPorts,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            url: String::new(),
            api_key: String::new(),
            server_type: ServerType::Zlmediakit,
            weight: 100,
            enabled: true,
            server_tag: String::new(),
            protocol_ports: ServerProtocolPorts::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}