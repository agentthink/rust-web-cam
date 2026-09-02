use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::recording::RecordingFormat;
use crate::domain::server::ServerProtocolPorts;

/// GB28181 流媒体配置
///
/// ## 视频配置 (video_*)
/// - video_codec: 视频编码格式。常见值:
///   - "PS" (默认): MPEG-PS, 海康大华常用
///   - "H264": 纯H.264, 适用于大部分设备
///   - "H265": H.265/HEVC, 部分新设备支持
///   - "JPEG": MJPEG
///
/// ## 音频配置 (audio_*)
/// - audio_codec: 音频编码格式。常见值:
///   - "PCMA" (默认): G.711 A-law
///   - "PCMU": G.711 μ-law  
///   - "AAC": AAC-LC, 部分设备支持
///   - "NONE"/"OFF": 禁用音频
///
/// ## RTP负载类型 (payload_type)
/// - video_payload_type: 视频RTP负载类型, 通常96-127。一般96即可
/// - audio_payload_type: 音频RTP负载类型, 8(G.711)是标准值
///
/// ## H.264特定 (profile_level_id, sprop_parameter_sets)
/// - profile_level_id: SPS/PPS的profile_level_id, 如"4D001F"(Main Profile)
/// - sprop_parameter_sets: Base64编码的SPS/PPS, 可从设备能力获取
///
/// ## PS封装特定 (packaging_mode)
/// - packaging_mode: "HIS" (默认) 表示96字节头封装
///
/// ## 传输模式 (stream_mode)
/// - stream_mode: "recvonly"(默认) 仅接收, "sendonly"仅发送, "sendrecv"双向
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// 视频编码格式: PS/H264/H265/JPEG, 默认"PS"
    #[serde(
        default = "default_video_codec",
        skip_serializing_if = "Option::is_none"
    )]
    pub video_codec: Option<String>,

    /// 音频编码格式: PCMA/PCMU/AAC/NONE, 默认"PCMA"
    #[serde(
        default = "default_audio_codec",
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_codec: Option<String>,

    /// 视频RTP负载类型, 默认96
    #[serde(default = "default_video_pt", skip_serializing_if = "Option::is_none")]
    pub video_payload_type: Option<u8>,

    /// 音频RTP负载类型, 默认8
    #[serde(default = "default_audio_pt", skip_serializing_if = "Option::is_none")]
    pub audio_payload_type: Option<u8>,

    /// H.264/H.265的profile_level_id, 如"4D001F"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_level_id: Option<String>,

    /// PS封装的packaging_mode, 默认"HIS"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packaging_mode: Option<String>,

    /// H.264的SPS/PPS参数, Base64编码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprop_parameter_sets: Option<String>,

    /// 传输模式: recvonly/sendonly/sendrecv, 默认"recvonly"
    #[serde(
        default = "default_stream_mode",
        skip_serializing_if = "Option::is_none"
    )]
    pub stream_mode: Option<String>,
}

fn default_video_codec() -> Option<String> {
    Some("PS".to_string())
}
fn default_audio_codec() -> Option<String> {
    Some("PCMA".to_string())
}
fn default_video_pt() -> Option<u8> {
    Some(96)
}
fn default_audio_pt() -> Option<u8> {
    Some(8)
}
fn default_stream_mode() -> Option<String> {
    Some("recvonly".to_string())
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            video_codec: default_video_codec(),
            audio_codec: default_audio_codec(),
            video_payload_type: default_video_pt(),
            audio_payload_type: default_audio_pt(),
            profile_level_id: Some("4D001F".to_string()),
            packaging_mode: Some("HIS".to_string()),
            sprop_parameter_sets: None,
            stream_mode: Some(default_stream_mode().unwrap()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub format: Option<RecordingFormat>,
    #[serde(default)]
    pub max_duration_secs: Option<u32>,
    #[serde(default)]
    pub max_file_size_mb: Option<u32>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            format: None,
            max_duration_secs: None,
            max_file_size_mb: None,
            labels: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Protocol {
    Gb28181,
    Onvif,
    Rtsp,
    Rtmp,
    Hls,
    WebRTC,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Gb28181 => write!(f, "GB28181"),
            Protocol::Onvif => write!(f, "ONVIF"),
            Protocol::Rtsp => write!(f, "RTSP"),
            Protocol::Rtmp => write!(f, "RTMP"),
            Protocol::Hls => write!(f, "HLS"),
            Protocol::WebRTC => write!(f, "WebRTC"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DeviceStatus {
    Offline,
    Online,
    Error,
    Maintaining,
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceStatus::Offline => write!(f, "offline"),
            DeviceStatus::Online => write!(f, "online"),
            DeviceStatus::Error => write!(f, "error"),
            DeviceStatus::Maintaining => write!(f, "maintaining"),
        }
    }
}

impl Default for DeviceStatus {
    fn default() -> Self {
        DeviceStatus::Offline
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DeviceType {
    NVR,
    IPC,
    DVR,
    Encoder,
    VideoServer,
    Camera,
    Platform,
    Other,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::NVR => write!(f, "NVR"),
            DeviceType::IPC => write!(f, "IPC"),
            DeviceType::DVR => write!(f, "DVR"),
            DeviceType::Encoder => write!(f, "Encoder"),
            DeviceType::VideoServer => write!(f, "VideoServer"),
            DeviceType::Camera => write!(f, "Camera"),
            DeviceType::Platform => write!(f, "Platform"),
            DeviceType::Other => write!(f, "Other"),
        }
    }
}

impl Default for DeviceType {
    fn default() -> Self {
        DeviceType::Other
    }
}

impl DeviceType {
    pub fn from_gb28181_code(code: &str) -> Self {
        if code.len() < 13 {
            return DeviceType::Other;
        }
        let type_code = &code[10..13];
        match type_code {
            "111" => DeviceType::DVR,
            "112" => DeviceType::VideoServer,
            "113" => DeviceType::Encoder,
            "118" => DeviceType::NVR,
            "130" => DeviceType::DVR,
            "131" => DeviceType::Camera,
            "132" => DeviceType::IPC,
            _ => {
                if let Ok(num) = type_code.parse::<u32>() {
                    if (200..=216).contains(&num) {
                        return DeviceType::Platform;
                    }
                }
                DeviceType::Other
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamProtocol {
    Rtsp,
    Rtmp,
    Gb28181,
    HttpFlv,
}

impl fmt::Display for StreamProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamProtocol::Rtsp => write!(f, "RTSP"),
            StreamProtocol::Rtmp => write!(f, "RTMP"),
            StreamProtocol::Gb28181 => write!(f, "GB28181"),
            StreamProtocol::HttpFlv => write!(f, "HTTP-FLV"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushUrl {
    pub protocol: StreamProtocol,
    pub url: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullUrl {
    pub protocol: StreamProtocol,
    pub url: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub protocol: Protocol,
    pub status: DeviceStatus,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,

    pub host: String,
    pub port: u16,

    pub device_username: Option<String>,
    pub device_password: Option<String>,

    pub push_urls: Vec<PushUrl>,
    pub pull_urls: Vec<PullUrl>,

    pub playback_username: Option<String>,
    pub playback_password: Option<String>,

    pub media_server_tag: Option<String>,
    pub app: Option<String>,

    pub device_tag: Option<String>,
    pub parent_device_tag: Option<String>,

    pub region_code: Option<String>,

    pub group_id: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended: Option<serde_json::Value>,

    #[serde(skip_serializing, default)]
    pub is_online: bool,

    #[serde(default)]
    pub has_stream: bool,

    #[serde(default)]
    pub device_type: DeviceType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type_code: Option<String>,

    #[serde(default)]
    pub channel_count: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_config: Option<StreamConfig>,
}

impl Device {
    pub fn new(name: String, protocol: Protocol) -> Self {
        Self {
            id: 0,
            name,
            protocol,
            status: DeviceStatus::Offline,
            is_public: false,
            created_at: Utc::now(),
            host: String::new(),
            port: 0,
            device_username: None,
            device_password: None,
            push_urls: Vec::new(),
            pull_urls: Vec::new(),
            playback_username: None,
            playback_password: None,
            media_server_tag: None,
            app: None,
            device_tag: None,
            parent_device_tag: None,
            region_code: None,
            group_id: None,
            extended: None,
            is_online: false,
            has_stream: false,
            device_type: DeviceType::Other,
            device_type_code: None,
            channel_count: 0,
            stream_config: None,
        }
    }

    pub fn set_online(&mut self) {
        self.status = DeviceStatus::Online;
        self.is_online = true;
    }

    pub fn set_offline(&mut self) {
        self.status = DeviceStatus::Offline;
        self.is_online = false;
    }

    pub fn select_source(&self) -> Option<(StreamProtocol, String)> {
        self.pull_urls
            .iter()
            .min_by_key(|p| p.priority)
            .map(|p| (p.protocol, p.url.clone()))
            .or_else(|| {
                self.push_urls
                    .iter()
                    .min_by_key(|p| p.priority)
                    .map(|p| (p.protocol, p.url.clone()))
            })
            .or_else(|| {
                self.extended
                    .as_ref()
                    .and_then(|e| e.get("rtsp_full_url"))
                    .and_then(|v| v.as_str())
                    .filter(|url| !url.is_empty())
                    .map(|url| (StreamProtocol::Rtsp, url.to_string()))
            })
    }

    pub fn is_push_mode(&self) -> bool {
        !self.push_urls.is_empty()
    }

    pub fn is_pull_mode(&self) -> bool {
        self.push_urls.is_empty() && !self.pull_urls.is_empty()
    }

    pub fn recording_config(&self) -> RecordingConfig {
        self.extended
            .as_ref()
            .and_then(|e| serde_json::from_value(e.clone()).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,

    #[serde(serialize_with = "serialize_uppercase")]
    pub device_tag: String,

    #[serde(serialize_with = "serialize_uppercase")]
    pub channel_tag: String,

    pub name: String,
    pub status: DeviceStatus,

    pub device_type: DeviceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_extended: Option<serde_json::Value>,

    #[serde(default)]
    pub is_default: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_channel_tag: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub civil_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(default)]
    pub parental: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

fn serialize_uppercase<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_uppercase())
}

impl Channel {
    pub fn new(device_tag: String, channel_tag: String, name: String) -> Self {
        Self {
            id: 0,
            device_tag,
            channel_tag,
            name,
            status: DeviceStatus::Offline,
            device_type: DeviceType::Other,
            device_type_code: None,
            channel_extended: None,
            is_default: false,
            parent_channel_tag: None,
            civil_code: None,
            address: None,
            ip_address: None,
            port: 0,
            manufacturer: None,
            model: None,
            parental: 0,
            extended: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn is_online(&self) -> bool {
        self.status == DeviceStatus::Online
    }

    pub fn is_directory(&self) -> bool {
        self.parental > 0
    }

    pub fn get_device_type_code(&self) -> Option<u32> {
        if self.channel_tag.len() >= 13 {
            self.channel_tag[10..13].parse().ok()
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
        self.is_voice_input_channel() || self.is_voice_output_channel()
    }

    pub fn is_ipc(&self) -> bool {
        self.device_type == DeviceType::IPC || self.device_type_code.as_deref() == Some("132")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelWithDevice {
    #[serde(flatten)]
    pub channel: Channel,
    pub device_name: Option<String>,
    pub device_status: Option<DeviceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayLinks {
    pub token: String,
    pub stream_id: String,
    pub expires_at: i64,
    pub ports: ServerProtocolPorts,
    pub rtsp_signaling: Option<String>,
    pub rtsp_media: Option<String>,
    pub flv: Option<String>,
    pub hls: Option<String>,
    pub webrtc: Option<String>,
    pub web_flv: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub protocol: Protocol,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    pub device_username: Option<String>,
    pub device_password: Option<String>,
    pub push_urls: Option<Vec<PushUrl>>,
    pub pull_urls: Option<Vec<PullUrl>>,
    pub playback_username: Option<String>,
    pub playback_password: Option<String>,
    pub media_server_tag: Option<String>,
    pub app: Option<String>,
    pub device_tag: Option<String>,
    pub parent_device_tag: Option<String>,
    pub region_code: Option<String>,
    pub is_public: Option<bool>,
    pub extended: Option<serde_json::Value>,
    #[serde(default)]
    pub device_type: Option<DeviceType>,
    pub device_type_code: Option<String>,
}

impl Default for CreateDeviceRequest {
    fn default() -> Self {
        Self {
            name: String::new(),
            protocol: Protocol::Rtsp,
            host: None,
            port: None,
            device_username: None,
            device_password: None,
            push_urls: None,
            pull_urls: None,
            playback_username: None,
            playback_password: None,
            media_server_tag: None,
            app: None,
            device_tag: None,
            parent_device_tag: None,
            region_code: None,
            is_public: Some(false),
            extended: None,
            device_type: None,
            device_type_code: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub device_username: Option<String>,
    pub device_password: Option<String>,
    pub push_urls: Option<Vec<PushUrl>>,
    pub pull_urls: Option<Vec<PullUrl>>,
    pub playback_username: Option<String>,
    pub playback_password: Option<String>,
    pub media_server_tag: Option<String>,
    pub app: Option<String>,
    pub device_tag: Option<String>,
    pub parent_device_tag: Option<String>,
    pub region_code: Option<String>,
    pub group_id: Option<i64>,
    pub is_public: Option<bool>,
    pub extended: Option<serde_json::Value>,
    pub status: Option<DeviceStatus>,
    pub device_type: Option<DeviceType>,
    pub device_type_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWithChildren {
    #[serde(flatten)]
    pub device: Device,
    pub children: Vec<DeviceWithChildren>,
}
