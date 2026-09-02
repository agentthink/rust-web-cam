use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: SignalEvent);
}
pub struct FnEventHandler<F> {
    f: F,  // 存储闭包
}
impl<F, Fut> FnEventHandler<F>
where
    F: Fn(SignalEvent) -> Fut + Send + Sync,  // F 是接收事件、返回 Future 的闭包
    Fut: std::future::Future<Output = ()> + Send,  // Future 必须可 Send
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}
#[async_trait]
impl<F, Fut> EventHandler for FnEventHandler<F>
where
    F: Fn(SignalEvent) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ()> + Send,
{
    async fn handle(&self, event: SignalEvent) {
        (self.f)(event).await;
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalEvent {
    DeviceRegister {
        device_id: i64,
        device_tag: Option<String>,
        name: String,
        stream_key: Option<String>,
        manufacturer: Option<String>,
        model: Option<String>,
        protocol: ProtocolType,
    },
    DeviceKeepalive {
        device_id: i64,
        device_tag: Option<String>,
        timestamp: DateTime<Utc>,
    },
    DeviceOffline {
        device_id: i64,
        device_tag: Option<String>,
        reason: Option<String>,
    },
    DeviceOnline {
        device_id: i64,
        device_tag: Option<String>,
    },
    StreamStart {
        device_id: i64,
        stream_key: String,
    },
    StreamStop {
        device_id: i64,
        stream_key: String,
        reason: Option<String>,
    },
    StreamRecover {
        device_id: i64,
        stream_key: String,
    },
    StreamRetriesExhausted {
        device_id: i64,
        stream_key: String,
        retry_count: u8,
        last_error: Option<String>,
    },
    StreamRestart {
        device_id: i64,
        stream_key: String,
    },
    StartPlay {
        device_id: i64,
        device_tag: Option<String>,
        session_id: String,
        channel_id: Option<String>,
        transport: TransportType,
        media_server_name: Option<String>,
    },
    StopPlay {
        device_id: i64,
        device_tag: Option<String>,
        session_id: String,
    },
    PtzControl {
        device_id: String,
        command: PtzCommand,
        speed: Option<u8>,
    },
    Alarm {
        device_id: String,
        alarm_type: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
    QueryDeviceInfo {
        device_id: i64,
        device_tag: Option<String>,
    },
    QueryDeviceStatus {
        device_id: i64,
        device_tag: Option<String>,
    },
    QueryDeviceConfig {
        device_id: i64,
        device_tag: Option<String>,
        config_type: String,
    },
    SetDeviceConfig {
        device_id: i64,
        device_tag: Option<String>,
        config_type: String,
        config_value: String,
    },
    PresetQuery {
        device_id: i64,
        device_tag: Option<String>,
        channel_id: String,
    },
    PresetSet {
        device_id: i64,
        device_tag: Option<String>,
        channel_id: String,
        preset_name: String,
    },
    PresetGoto {
        device_id: i64,
        device_tag: Option<String>,
        channel_id: String,
        preset_index: u32,
    },
    PresetRemove {
        device_id: i64,
        device_tag: Option<String>,
        channel_id: String,
        preset_index: u32,
    },
    CatalogResponse {
        device_id: i64,
        device_tag: Option<String>,
        channels: Vec<CatalogChannel>,
    },
    MediaServerOnline {
        media_server_tag: String,
    },
    MediaServerOffline {
        media_server_tag: String,
        reason: Option<String>,
    },
    DeviceSubscribe {
        device_id: i64,
        device_tag: Option<String>,
        event_types: Vec<String>,
        expires: u32,
    },
    DeviceNotify {
        device_id: i64,
        device_tag: Option<String>,
        event_type: String,
        expires: u32,
    },
    CatalogNotify {
        device_id: i64,
        device_tag: Option<String>,
        event_type: String,
        channels: Vec<CatalogChannel>,
    },
    StartAudioTalk {
        device_id: i64,
        device_tag: Option<String>,
        audio_port: u16,
    },
    StopAudioTalk {
        device_id: i64,
        device_tag: Option<String>,
    },
    AudioData {
        device_id: i64,
        device_tag: Option<String>,
        data: Vec<u8>,
        timestamp: u32,
    },
}
impl SignalEvent {
    /// 获取事件类型字符串
    pub fn event_type(&self) -> String {
        match self {
            SignalEvent::DeviceRegister { .. } => "device_register".to_string(),
            SignalEvent::DeviceKeepalive { .. } => "device_keepalive".to_string(),
            SignalEvent::DeviceOffline { .. } => "device_offline".to_string(),
            SignalEvent::DeviceOnline { .. } => "device_online".to_string(),
            SignalEvent::StreamStart { .. } => "stream_start".to_string(),
            SignalEvent::StreamStop { .. } => "stream_stop".to_string(),
            SignalEvent::StreamRecover { .. } => "stream_recover".to_string(),
            SignalEvent::StreamRetriesExhausted { .. } => "stream_retries_exhausted".to_string(),
            SignalEvent::StreamRestart { .. } => "stream_restart".to_string(),
            SignalEvent::StartPlay { .. } => "start_play".to_string(),
            SignalEvent::StopPlay { .. } => "stop_play".to_string(),
            SignalEvent::PtzControl { .. } => "ptz_control".to_string(),
            SignalEvent::Alarm { .. } => "alarm".to_string(),
            SignalEvent::QueryDeviceInfo { .. } => "query_device_info".to_string(),
            SignalEvent::QueryDeviceStatus { .. } => "query_device_status".to_string(),
            SignalEvent::QueryDeviceConfig { .. } => "query_device_config".to_string(),
            SignalEvent::SetDeviceConfig { .. } => "set_device_config".to_string(),
            SignalEvent::PresetQuery { .. } => "preset_query".to_string(),
            SignalEvent::PresetSet { .. } => "preset_set".to_string(),
            SignalEvent::PresetGoto { .. } => "preset_goto".to_string(),
            SignalEvent::PresetRemove { .. } => "preset_remove".to_string(),
            SignalEvent::CatalogResponse { .. } => "catalog_response".to_string(),
            SignalEvent::MediaServerOnline { .. } => "media_server_online".to_string(),
            SignalEvent::MediaServerOffline { .. } => "media_server_offline".to_string(),
            SignalEvent::DeviceSubscribe { .. } => "device_subscribe".to_string(),
            SignalEvent::DeviceNotify { .. } => "device_notify".to_string(),
            SignalEvent::CatalogNotify { .. } => "catalog_notify".to_string(),
            SignalEvent::StartAudioTalk { .. } => "start_audio_talk".to_string(),
            SignalEvent::StopAudioTalk { .. } => "stop_audio_talk".to_string(),
            SignalEvent::AudioData { .. } => "audio_data".to_string(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogChannel {
    pub device_id: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub parental: bool,
    pub parent_id: Option<String>,
    pub civil_code: Option<String>,
    pub address: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<u16>,
    pub owner: Option<String>,
    pub secrecy: Option<u8>,
    pub device_type: Option<String>,
    pub ptz_type: Option<u8>,
    pub info: Option<DeviceInfoBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoBlock {
    pub device_type: Option<String>,
    pub protocol: Option<String>,
    pub ptz_type: Option<u8>,
    pub video_input_number: Option<u8>,
    pub audio_input_number: Option<u8>,
    pub alarm_output_number: Option<u8>,
}

impl CatalogChannel {
    pub fn is_online(&self) -> bool {
        self.status == "ON"
    }

    pub fn is_directory(&self) -> bool {
        self.parental
    }

    pub fn get_device_type_code(&self) -> Option<u32> {
        if self.device_id.len() >= 13 {
            self.device_id[8..12].parse().ok()
        } else {
            None
        }
    }

    pub fn is_voice_input_channel(&self) -> bool {
        self.get_device_type_code() == Some(136)
    }

    pub fn is_voice_output_channel(&self) -> bool {
        self.get_device_type_code() == Some(137)
    }

    pub fn is_audio_channel(&self) -> bool {
        match self.get_device_type_code() {
            Some(136) | Some(137) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProtocolType {
    Gb28181,
    Onvif,
    Rtsp,
    WebRtc,
    Custom(String),
}

impl std::fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolType::Gb28181 => write!(f, "GB28181"),
            ProtocolType::Onvif => write!(f, "ONVIF"),
            ProtocolType::Rtsp => write!(f, "RTSP"),
            ProtocolType::WebRtc => write!(f, "WebRTC"),
            ProtocolType::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtzCommand {
    Up,
    Down,
    Left,
    Right,
    ZoomIn,
    ZoomOut,
    FocusIn,
    FocusOut,
    Stop,
    ContinuousMove { pan: f64, tilt: f64, zoom: f64 },
    AbsoluteMove { pan: f64, tilt: f64, zoom: f64 },
    RelativeMove { pan: f64, tilt: f64, zoom: f64 },
    GotoPreset { preset_token: String },
    SetPreset { preset_name: Option<String> },
    RemovePreset { preset_token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransportType {
    UDP,
    TCP,
    HTTP,
    WebSocket,
}
