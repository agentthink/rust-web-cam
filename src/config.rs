use anyhow::Context;
use serde::Deserialize;
use std::path::Path;

use crate::auth::config::JwtConfig;

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct LogConfig {
    pub level: String,
    #[serde(default)]
    pub sql: bool,
    #[serde(default)]
    pub http: bool,
    #[serde(default)]
    pub media_server: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            sql: false,
            http: false,
            media_server: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub media_servers: MediaServersConfig,
    pub cluster: ClusterConfig,
    pub session: SessionConfig,
    pub onvif: OnvifConfig,
    pub rtsp_auth: RtspAuthConfig,
    pub signaling_server: SignalingServerConfig,
    pub jwt: JwtConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub recovery: RecoveryConfig,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_cert: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub debug_sql: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct MediaServersConfig {
    pub servers: Vec<MediaServerConfig>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct MediaServerConfig {
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub server_type: String,
    pub weight: u32,
    pub enabled: bool,
    #[serde(default)]
    pub server_tag: String,
    #[serde(default)]
    pub protocol_ports: crate::domain::server::ServerProtocolPorts,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct MediaPortsConfig {
    pub rtsp_signaling: u16,
    pub rtsp_media: u16,
    pub rtmp: u16,
    pub http_flv: u16,
    pub ws_flv: u16,
}

impl MediaPortsConfig {
    pub fn from_config(config: &MediaPortsConfig) -> Self {
        config.clone()
    }
}

impl Default for MediaPortsConfig {
    fn default() -> Self {
        Self {
            rtsp_signaling: 8554,
            rtsp_media: 8555,
            rtmp: 1935,
            http_flv: 8080,
            ws_flv: 8080,
        }
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ClusterConfig {
    pub strategy: String,
    pub health_check_interval_secs: u64,
    pub max_session_per_server: u32,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct RecoveryConfig {
    /// 定期检查已离线流的间隔秒数（默认 30）
    pub check_interval_secs: u64,
    /// 重试退避基础秒数，采用指数退避 5s→10s→20s→40s→...（默认 5）
    pub base_backoff_secs: u64,
    /// 单次重试最大退避秒数上限（默认 300，即 5 分钟）
    pub max_backoff_secs: u64,
    /// 设备离线后最大自动重试次数，超过后进入终态 Error 需要人工介入（默认 20）
    pub max_retries: u8,
    /// 同时允许的最大并发恢复任务数，防止媒体服务器被突发请求压垮（默认 10）
    pub max_concurrent_recoveries: u32,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            base_backoff_secs: 5,
            max_backoff_secs: 300,
            max_retries: 20,
            max_concurrent_recoveries: 10,
        }
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct SessionConfig {
    pub expiration_secs: i64,
    pub max_per_user: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            expiration_secs: 300,
            max_per_user: 10,
        }
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct OnvifConfig {
    pub discovery_enabled: bool,
    pub scopes: Vec<String>,
}

impl Default for OnvifConfig {
    fn default() -> Self {
        Self {
            discovery_enabled: true,
            scopes: vec![
                "onvif://www.onvif.org/Profile/Streaming".to_string(),
                "onvif://www.onvif.org/Profile/PTZ".to_string(),
                "onvif://www.onvif.org/Profile/Events".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct RtspAuthConfig {
    pub enabled: bool,
    pub realm: String,
    pub default_username: Option<String>,
    pub default_password: Option<String>,
}

impl Default for RtspAuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            realm: "RustCam".to_string(),
            default_username: Some("admin".to_string()),
            default_password: Some("admin".to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct SignalingServerConfig {
    pub server_gb_id: String,
    pub server_gb_domain: String,
    pub bind_ip: String,
    pub tcp_signaling_port: u16,
    pub udp_signaling_port: u16,
    pub tcp_audio_port_start: u16,
    pub tcp_audio_port_end: u16,
    pub tcp_audio_idle_timeout_secs: u64,
    pub tcp_audio_max_servers: usize,
}

impl Default for SignalingServerConfig {
    fn default() -> Self {
        Self {
            server_gb_id: "31011500001000000001".to_string(),
            server_gb_domain: "3101150000".to_string(),
            bind_ip: "0.0.0.0".to_string(),
            tcp_signaling_port: 5060,
            udp_signaling_port: 5061,
            tcp_audio_port_start: 15000,
            tcp_audio_port_end: 15100,
            tcp_audio_idle_timeout_secs: 30,
            tcp_audio_max_servers: 10,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://postgres:123456@127.0.0.1:5435/rustcam".to_string(),
            max_connections: 10,
            debug_sql: Some(false),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
        }
    }
}

impl Default for MediaServersConfig {
    fn default() -> Self {
        Self {
            servers: vec![MediaServerConfig {
                name: "zlmediakit-1".to_string(),
                url: "http://localhost:8081".to_string(),
                api_key: "0123456789abcdef".to_string(),
                server_type: "zlmediakit".to_string(),
                weight: 100,
                enabled: true,
                server_tag: String::new(),
                protocol_ports: crate::domain::server::ServerProtocolPorts::default(),
            }],
        }
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            strategy: "round_robin".to_string(),
            health_check_interval_secs: 30,
            max_session_per_server: 1000,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            media_servers: MediaServersConfig::default(),
            cluster: ClusterConfig::default(),
            session: SessionConfig::default(),
            onvif: OnvifConfig::default(),
            rtsp_auth: RtspAuthConfig::default(),
            signaling_server: SignalingServerConfig::default(),
            jwt: JwtConfig::default(),
            log: LogConfig::default(),
            recovery: RecoveryConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        Self::load_from_file("config.toml")
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let default_config = AppConfig::default();
            default_config.save_to_file(path)?;
            tracing::info!("Created default config at {:?}", path);
            return Ok(default_config);
        }

        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .build()
            .with_context(|| format!("Failed to load config from {:?}", path))?;

        let config: AppConfig = settings
            .try_deserialize()
            .context("Failed to deserialize config")?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }
}
