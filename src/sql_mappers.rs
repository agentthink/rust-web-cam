use rbatis::html_sql;
use rbatis::rbdc::datetime::DateTime as RbatisDateTime;
use rbatis::sql;
use crate::domain::server::{Server, ServerType};
use crate::domain::{Device, Protocol, DeviceStatus, DeviceType, PushUrl, PullUrl, Recording, RecordingState, RecordingFormat, Stream, StreamState, StreamConfig, Channel};
use crate::domain::stream::make_stream_key;
use crate::domain::region::RegionRow;
use crate::error::AppError;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeviceRow {
    pub id: i64,
    pub name: String,
    pub protocol: String,
    pub status: String,
    pub host: String,
    pub port: Option<i32>,
    pub rtsp_url: Option<String>,
    pub device_tag: Option<String>,
    pub parent_device_tag: Option<String>,
    pub is_public: bool,
    pub created_at: RbatisDateTime,
    pub device_password: Option<String>,
    pub playback_username: Option<String>,
    pub playback_password: Option<String>,
    pub media_server_tag: Option<String>,
    pub app: Option<String>,
    pub push_urls: Option<serde_json::Value>,
    pub pull_urls: Option<serde_json::Value>,
    pub region_code: Option<String>,
    pub updated_at: Option<RbatisDateTime>,
    pub ssrc: Option<String>,
    pub gb_username: Option<String>,
    pub gb_password: Option<String>,
    pub extended: Option<serde_json::Value>,
    pub group_id: Option<i64>,
    pub device_type: Option<String>,
    pub device_type_code: Option<String>,
    pub channel_count: Option<i32>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChannelRow {
    pub id: i64,
    pub device_tag: String,
    pub channel_tag: String,
    pub name: String,
    pub status: String,
    pub device_type: Option<String>,
    pub device_type_code: Option<String>,
    pub channel_extended: Option<serde_json::Value>,
    pub is_default: Option<bool>,
    pub parent_channel_tag: Option<String>,
    pub civil_code: Option<String>,
    pub address: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub parental: Option<i32>,
    pub extended: Option<serde_json::Value>,
    pub created_at: RbatisDateTime,
    pub updated_at: Option<RbatisDateTime>,
}

impl TryFrom<DeviceRow> for Device {
    type Error = AppError;

    fn try_from(row: DeviceRow) -> Result<Self, Self::Error> {
        let protocol = match row.protocol.to_lowercase().as_str() {
            "gb28181" => Protocol::Gb28181,
            "onvif" => Protocol::Onvif,
            "rtsp" => Protocol::Rtsp,
            "rtmp" => Protocol::Rtmp,
            "hls" => Protocol::Hls,
            "webrtc" => Protocol::WebRTC,
            _ => return Err(AppError::Internal(format!("Unknown protocol: {}", row.protocol))),
        };

        let status = match row.status.to_lowercase().as_str() {
            "online" => DeviceStatus::Online,
            "error" => DeviceStatus::Error,
            "maintaining" => DeviceStatus::Maintaining,
            _ => DeviceStatus::Offline,
        };

        let mut ext = row.extended.clone().unwrap_or(serde_json::Value::Object(Default::default()));
        let ext_obj = ext.as_object().cloned().unwrap_or_default();

        // 合并协议特有字段到 extended（保证数据不丢失）
        if let Some(obj) = ext.as_object_mut() {
            if row.rtsp_url.is_some() {
                obj.insert("rtsp_url".to_string(), serde_json::json!(row.rtsp_url));
            }
            if row.gb_username.is_some() {
                obj.insert("gb_username".to_string(), serde_json::json!(row.gb_username));
            }
            if row.gb_password.is_some() {
                obj.insert("gb_password".to_string(), serde_json::json!(row.gb_password));
            }
            if row.ssrc.is_some() && !obj.contains_key("ssrc") {
                obj.insert("ssrc".to_string(), serde_json::json!(row.ssrc));
            }
        }

        let get_str = |key: &str| ext_obj.get(key).and_then(|v| v.as_str().map(String::from));
        let get_push_urls = || -> Vec<PushUrl> {
            row.push_urls.as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .or_else(|| ext_obj.get("push_urls").and_then(|v| serde_json::from_value(v.clone()).ok()))
                .unwrap_or_default()
        };
        let get_pull_urls = || -> Vec<PullUrl> {
            row.pull_urls.as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .or_else(|| ext_obj.get("pull_urls").and_then(|v| serde_json::from_value(v.clone()).ok()))
                .unwrap_or_default()
        };
        let get_stream_config = || -> Option<StreamConfig> {
            ext_obj.get("stream_config")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        };

        let created_at = chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);

        let device_type = row.device_type.as_ref()
            .map(|dt| match dt.as_str() {
                "NVR" => DeviceType::NVR,
                "IPC" => DeviceType::IPC,
                "DVR" => DeviceType::DVR,
                "Encoder" => DeviceType::Encoder,
                "VideoServer" => DeviceType::VideoServer,
                "Camera" => DeviceType::Camera,
                "Platform" => DeviceType::Platform,
                _ => DeviceType::Other,
            })
            .unwrap_or(DeviceType::Other);

        let device = Device {
            id: row.id,
            name: row.name,
            protocol,
            status,
            is_public: row.is_public,
            created_at,
            host: if row.host.is_empty() { get_str("host").unwrap_or_default() } else { row.host },
            port: row.port.unwrap_or(0) as u16,
            device_username: get_str("device_username"),
            device_password: row.device_password.or_else(|| get_str("device_password")),
            push_urls: get_push_urls(),
            pull_urls: get_pull_urls(),
            playback_username: row.playback_username.or_else(|| get_str("playback_username")),
            playback_password: row.playback_password.or_else(|| get_str("playback_password")),
            media_server_tag: row.media_server_tag.or_else(|| get_str("media_server_tag")),
            app: row.app.or_else(|| get_str("app")),
            device_tag: row.device_tag,
            parent_device_tag: row.parent_device_tag,
            region_code: row.region_code.or_else(|| get_str("region_code")),
            group_id: row.group_id,
            extended: ext.into(),
            is_online: false,
            has_stream: false,
            device_type,
            device_type_code: row.device_type_code,
            channel_count: row.channel_count.unwrap_or(0),
            stream_config: get_stream_config(),
        };
        Ok(device)
    }
}

impl Device {
    pub fn extended_json(&self) -> serde_json::Value {
        serde_json::json!({
            "device_username": self.device_username,
        })
    }

    pub fn stream_url(&self) -> Option<String> {
        self.select_source().map(|(_, url)| url)
    }
}

#[html_sql("sql/device.html")]
pub async fn device_select_all(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<Vec<DeviceRow>> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_select_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<Vec<DeviceRow>> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_insert(
    rb: &dyn rbatis::Executor,
    name: Option<&str>,
    protocol: Option<&str>,
    status: Option<&str>,
    host: Option<&str>,
    port: Option<i32>,
    rtsp_url: Option<&str>,
    device_tag: Option<&str>,
    parent_device_tag: Option<&str>,
    is_public: Option<bool>,
    created_at: Option<RbatisDateTime>,
    device_password: Option<&str>,
    playback_username: Option<&str>,
    playback_password: Option<&str>,
    media_server_tag: Option<&str>,
    push_urls: Option<serde_json::Value>,
    pull_urls: Option<serde_json::Value>,
    region_code: Option<&str>,
    updated_at: Option<RbatisDateTime>,
    ssrc: Option<&str>,
    extended: Option<serde_json::Value>,
    group_id: Option<i64>,
    app: Option<&str>,
    device_type: Option<&str>,
    device_type_code: Option<&str>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_update(
    rb: &dyn rbatis::Executor,
    id: i64,
    name: Option<&str>,
    protocol: Option<&str>,
    status: Option<&str>,
    host: Option<&str>,
    port: Option<i32>,
    rtsp_url: Option<&str>,
    device_tag: Option<&str>,
    parent_device_tag: Option<&str>,
    is_public: Option<bool>,
    device_password: Option<&str>,
    playback_username: Option<&str>,
    playback_password: Option<&str>,
    media_server_tag: Option<&str>,
    push_urls: Option<serde_json::Value>,
    pull_urls: Option<serde_json::Value>,
    region_code: Option<&str>,
    updated_at: Option<RbatisDateTime>,
    ssrc: Option<&str>,
    gb_username: Option<&str>,
    gb_password: Option<&str>,
    extended: Option<serde_json::Value>,
    group_id: Option<i64>,
    app: Option<&str>,
    device_type: Option<&str>,
    device_type_code: Option<&str>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_delete_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_delete_by_device_tag(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_update_by_device_tag(
    rb: &dyn rbatis::Executor,
    device_tag_query: &str,
    name: Option<&str>,
    protocol: Option<&str>,
    status: Option<&str>,
    host: Option<&str>,
    port: Option<i32>,
    rtsp_url: Option<&str>,
    device_tag: Option<&str>,
    parent_device_tag: Option<&str>,
    is_public: Option<bool>,
    device_password: Option<&str>,
    playback_username: Option<&str>,
    playback_password: Option<&str>,
    media_server_tag: Option<&str>,
    push_urls: Option<serde_json::Value>,
    pull_urls: Option<serde_json::Value>,
    region_code: Option<&str>,
    updated_at: Option<RbatisDateTime>,
    ssrc: Option<&str>,
    gb_username: Option<&str>,
    gb_password: Option<&str>,
    extended: Option<serde_json::Value>,
    group_id: Option<i64>,
    app: Option<&str>,
    device_type: Option<&str>,
    device_type_code: Option<&str>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_select_online_paginated(
    rb: &dyn rbatis::Executor,
    limit: i64,
    offset: i64,
) -> rbatis::Result<Vec<DeviceRow>> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_count_online(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<i64> {
    impled!()
}

#[html_sql("sql/stream.html")]
pub async fn stream_select_active_paginated(
    rb: &dyn rbatis::Executor,
    limit: i64,
    offset: i64,
) -> rbatis::Result<Vec<StreamRow>> {
    impled!()
}

#[html_sql("sql/stream.html")]
pub async fn stream_count_active(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<i64> {
    impled!()
}

#[html_sql("sql/session.html")]
pub async fn session_select_active_paginated(
    rb: &dyn rbatis::Executor,
    limit: i64,
    offset: i64,
) -> rbatis::Result<Vec<SessionRow>> {
    impled!()
}

#[html_sql("sql/session.html")]
pub async fn session_count_active(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<i64> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_select_all_paginated(
    rb: &dyn rbatis::Executor,
    limit: i64,
    offset: i64,
) -> rbatis::Result<Vec<DeviceRow>> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_count_top_level(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<i64> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_select_by_parent_device_tag(
    rb: &dyn rbatis::Executor,
    parent_device_tag: &str,
) -> rbatis::Result<Vec<DeviceRow>> {
    impled!()
}

#[html_sql("sql/device.html")]
pub async fn device_select_by_device_tag(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
) -> rbatis::Result<Vec<DeviceRow>> {
    impled!()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerRow {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub server_type: String,
    pub weight: i32,
    pub enabled: bool,
    pub server_tag: String,
    pub protocol_ports: serde_json::Value,
    pub created_at: RbatisDateTime,
    pub updated_at: RbatisDateTime,
}

impl TryFrom<ServerRow> for Server {
    type Error = AppError;

    fn try_from(row: ServerRow) -> Result<Self, Self::Error> {
        let server_type = ServerType::from_str(&row.server_type)
            .ok_or_else(|| AppError::Internal(format!("unknown server_type: {}", row.server_type)))?;
        Ok(Server {
            id: row.id,
            name: row.name,
            url: row.url,
            api_key: row.api_key,
            server_type,
            weight: row.weight as u32,
            enabled: row.enabled,
            server_tag: row.server_tag,
            protocol_ports: serde_json::from_value(row.protocol_ports).unwrap_or_default(),
            created_at: chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
                .unwrap_or_else(chrono::Utc::now),
            updated_at: chrono::DateTime::from_timestamp(row.updated_at.unix_timestamp(), 0)
                .unwrap_or_else(chrono::Utc::now),
        })
    }
}

#[html_sql("sql/server.html")]
pub async fn server_select_all(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<Vec<ServerRow>> {
    impled!()
}

#[html_sql("sql/server.html")]
pub async fn server_select_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<Option<ServerRow>> {
    impled!()
}

#[html_sql("sql/server.html")]
pub async fn server_select_by_tag(
    rb: &dyn rbatis::Executor,
    server_tag: &str,
) -> rbatis::Result<Vec<ServerRow>> {
    impled!()
}

#[html_sql("sql/server.html")]
pub async fn server_select_by_name(
    rb: &dyn rbatis::Executor,
    name: &str,
) -> rbatis::Result<Vec<ServerRow>> {
    impled!()
}

#[html_sql("sql/server.html")]
pub async fn server_insert(
    rb: &dyn rbatis::Executor,
    name: Option<&str>,
    url: Option<&str>,
    api_key: Option<&str>,
    server_type: Option<&str>,
    weight: Option<i32>,
    enabled: Option<bool>,
    server_tag: Option<&str>,
    protocol_ports: Option<serde_json::Value>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/server.html")]
pub async fn server_update(
    rb: &dyn rbatis::Executor,
    id: i64,
    name: &str,
    url: &str,
    api_key: &str,
    server_type: &str,
    weight: i32,
    enabled: bool,
    server_tag: &str,
    protocol_ports: serde_json::Value,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/server.html")]
pub async fn server_delete_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SessionRow {
    pub id: i64,
    pub session_type: String,
    pub device_tag: Option<String>,
    pub channel_tag: Option<String>,
    pub user_id: String,
    pub state: String,
    pub client_ip: Option<String>,
    pub client_type: Option<String>,
    pub media_server_tag: Option<String>,
    pub protocol: Option<String>,
    pub created_at: RbatisDateTime,
    pub last_activity: RbatisDateTime,
    pub expires_at: Option<RbatisDateTime>,
    pub bytes_sent: i64,
    pub bytes_received: i64,
}

#[sql("SELECT id, session_type, device_tag, channel_tag, user_id, state, client_ip, client_type, media_server_tag, protocol, created_at, last_activity, expires_at, bytes_sent, bytes_received FROM sessions ORDER BY created_at DESC")]
pub async fn session_select_all(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<Vec<SessionRow>> {
    impled!()
}

#[sql("SELECT id, session_type, device_tag, channel_tag, user_id, state, client_ip, client_type, media_server_tag, protocol, created_at, last_activity, expires_at, bytes_sent, bytes_received FROM sessions WHERE id = $1")]
pub async fn session_select_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<Vec<SessionRow>> {
    impled!()
}

#[html_sql("sql/session.html")]
pub async fn session_insert(
    rb: &dyn rbatis::Executor,
    session_type: Option<&str>,
    device_tag: Option<&str>,
    channel_tag: Option<&str>,
    user_id: Option<&str>,
    state: Option<&str>,
    client_ip: Option<&str>,
    client_type: Option<&str>,
    media_server_tag: Option<&str>,
    protocol: Option<&str>,
    created_at: Option<RbatisDateTime>,
    last_activity: Option<RbatisDateTime>,
    expires_at: Option<RbatisDateTime>,
    bytes_sent: Option<i64>,
    bytes_received: Option<i64>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/session.html")]
pub async fn session_update(
    rb: &dyn rbatis::Executor,
    session_type: Option<&str>,
    device_tag: Option<&str>,
    channel_tag: Option<&str>,
    user_id: Option<&str>,
    state: Option<&str>,
    client_ip: Option<&str>,
    client_type: Option<&str>,
    media_server_tag: Option<&str>,
    protocol: Option<&str>,
    created_at: Option<RbatisDateTime>,
    last_activity: Option<RbatisDateTime>,
    expires_at: Option<RbatisDateTime>,
    bytes_sent: Option<i64>,
    bytes_received: Option<i64>,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[sql("DELETE FROM sessions WHERE id = $1")]
pub async fn session_delete_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RecordingRow {
    pub id: i64,
    pub device_tag: Option<String>,
    pub channel_tag: Option<String>,
    pub media_server_name: String,
    pub state: String,
    pub format: String,
    pub output_path: Option<String>,
    pub file_size: i64,
    pub duration_secs: i64,
    pub created_at: RbatisDateTime,
    pub started_at: Option<RbatisDateTime>,
    pub stopped_at: Option<RbatisDateTime>,
    pub error_message: Option<String>,
    pub labels: Option<String>,
    pub filename: Option<String>,
}

impl TryFrom<RecordingRow> for Recording {
    type Error = AppError;

    fn try_from(row: RecordingRow) -> Result<Self, Self::Error> {
        let state = match row.state.to_lowercase().as_str() {
            "starting" => RecordingState::Starting,
            "recording" => RecordingState::Recording,
            "paused" => RecordingState::Paused,
            "stopping" => RecordingState::Stopping,
            "completed" => RecordingState::Completed,
            "error" => RecordingState::Error,
            _ => return Err(AppError::Internal(format!("Unknown recording state: {}", row.state))),
        };

        let format = match row.format.to_lowercase().as_str() {
            "hls" => RecordingFormat::Hls,
            "mp4" => RecordingFormat::Mp4,
            "flv" => RecordingFormat::Flv,
            "ts" => RecordingFormat::Ts,
            _ => return Err(AppError::Internal(format!("Unknown recording format: {}", row.format))),
        };

        let created_at = chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);

        let started_at = row.started_at.and_then(|s| chrono::DateTime::from_timestamp(s.unix_timestamp(), 0));

        let stopped_at = row.stopped_at.and_then(|s| chrono::DateTime::from_timestamp(s.unix_timestamp(), 0));

        let labels: Vec<String> = row.labels
            .as_ref()
            .and_then(|l| serde_json::from_str(l).ok())
            .unwrap_or_default();

        Ok(Recording {
            id: row.id,
            device_tag: row.device_tag,
            channel_tag: row.channel_tag,
            media_server_name: row.media_server_name,
            state,
            format,
            output_path: row.output_path,
            file_size: row.file_size as u64,
            duration_secs: row.duration_secs as u64,
            created_at,
            started_at,
            stopped_at,
            error_message: row.error_message,
            labels,
            filename: row.filename,
        })
    }
}

impl Recording {
    pub fn to_row(&self) -> RecordingRow {
        let state_str = self.state.to_string();
        let format_str = self.format.to_string();
        let labels_json = serde_json::to_string(&self.labels).ok();

        RecordingRow {
            id: self.id,
            device_tag: self.device_tag.clone(),
            channel_tag: self.channel_tag.clone(),
            media_server_name: self.media_server_name.clone(),
            state: state_str,
            format: format_str,
            output_path: self.output_path.clone(),
            file_size: self.file_size as i64,
            duration_secs: self.duration_secs as i64,
            created_at: RbatisDateTime::from_timestamp_millis(self.created_at.timestamp_millis()),
            started_at: self.started_at.map(|dt| RbatisDateTime::from_timestamp_millis(dt.timestamp_millis())),
            stopped_at: self.stopped_at.map(|dt| RbatisDateTime::from_timestamp_millis(dt.timestamp_millis())),
            error_message: self.error_message.clone(),
            labels: labels_json,
            filename: self.filename.clone(),
        }
    }
}

#[sql("SELECT id, device_tag, channel_tag, media_server_name, state, format, output_path, file_size, duration_secs, created_at, started_at, stopped_at, error_message, labels, filename FROM recordings ORDER BY created_at DESC")]
pub async fn recording_select_all(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<Vec<RecordingRow>> {
    impled!()
}

#[sql("SELECT id, device_tag, channel_tag, media_server_name, state, format, output_path, file_size, duration_secs, created_at, started_at, stopped_at, error_message, labels, filename FROM recordings ORDER BY created_at DESC LIMIT $1 OFFSET $2")]
pub async fn recording_select_paginated(
    rb: &dyn rbatis::Executor,
    limit: i64,
    offset: i64,
) -> rbatis::Result<Vec<RecordingRow>> {
    impled!()
}

#[sql("SELECT COUNT(*) FROM recordings")]
pub async fn recording_count_all(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<i64> {
    impled!()
}

#[sql("SELECT id, device_tag, channel_tag, media_server_name, state, format, output_path, file_size, duration_secs, created_at, started_at, stopped_at, error_message, labels, filename FROM recordings WHERE id = $1")]
pub async fn recording_select_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<Vec<RecordingRow>> {
    impled!()
}

#[sql("SELECT id, device_tag, channel_tag, media_server_name, state, format, output_path, file_size, duration_secs, created_at, started_at, stopped_at, error_message, labels, filename FROM recordings WHERE device_tag = $1 ORDER BY created_at DESC")]
pub async fn recording_select_by_device_tag(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
) -> rbatis::Result<Vec<RecordingRow>> {
    impled!()
}

#[html_sql("sql/recording.html")]
pub async fn recording_insert(
    rb: &dyn rbatis::Executor,
    id: i64,
    device_tag: Option<&str>,
    channel_tag: Option<&str>,
    media_server_name: &str,
    state: &str,
    format: &str,
    output_path: Option<String>,
    file_size: i64,
    duration_secs: i64,
    created_at: RbatisDateTime,
    started_at: Option<RbatisDateTime>,
    stopped_at: Option<RbatisDateTime>,
    error_message: Option<String>,
    labels: Option<String>,
    filename: Option<String>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[html_sql("sql/recording.html")]
pub async fn recording_update(
    rb: &dyn rbatis::Executor,
    device_tag: Option<&str>,
    channel_tag: Option<&str>,
    media_server_name: Option<String>,
    state: Option<String>,
    format: Option<String>,
    output_path: Option<String>,
    file_size: Option<i64>,
    duration_secs: Option<i64>,
    started_at: Option<RbatisDateTime>,
    stopped_at: Option<RbatisDateTime>,
    error_message: Option<String>,
    labels: Option<String>,
    filename: Option<String>,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[sql("DELETE FROM recordings WHERE id = $1")]
pub async fn recording_delete_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct StreamRow {
    pub id: i64,
    pub device_tag: Option<String>,
    pub channel_tag: Option<String>,
    pub media_server_tag: Option<String>,
    pub app: String,
    pub token: Option<String>,
    pub state: String,
    pub retry_count: Option<i32>,
    pub max_retries: Option<i32>,
    pub last_error: Option<String>,
    pub viewer_count: Option<i32>,
    pub bandwidth_in: Option<i64>,
    pub bandwidth_out: Option<i64>,
    pub last_keepalive_at: RbatisDateTime,
    pub created_at: RbatisDateTime,
}

impl TryFrom<StreamRow> for Stream {
    type Error = AppError;

    fn try_from(row: StreamRow) -> Result<Self, Self::Error> {
        let state = match row.state.to_lowercase().as_str() {
            "idle" => StreamState::Idle,
            "starting" => StreamState::Starting,
            "recovering" => StreamState::Recovering,
            "active" => StreamState::Active,
            "stopping" => StreamState::Stopping,
            "stopped" => StreamState::Stopped,
            "error" => StreamState::Error,
            _ => return Err(AppError::Internal(format!("Unknown stream state: {}", row.state))),
        };

        let last_keepalive_at = chrono::DateTime::from_timestamp(row.last_keepalive_at.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);
        let created_at = chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);

        Ok(Stream {
            id: row.id,
            device_tag: row.device_tag,
            channel_tag: row.channel_tag,
            media_server_tag: row.media_server_tag.unwrap_or_default(),
            app: row.app,
            token: row.token.unwrap_or_default(),
            state,
            retry_count: row.retry_count.unwrap_or(0) as u8,
            max_retries: row.max_retries.unwrap_or(5) as u8,
            last_error: row.last_error.clone(),
            viewer_count: row.viewer_count.unwrap_or(0) as u32,
            bandwidth_in: row.bandwidth_in.unwrap_or(0) as u64,
            bandwidth_out: row.bandwidth_out.unwrap_or(0) as u64,
            last_keepalive_at,
            created_at,
        })
    }
}

impl Stream {
    pub fn to_row(&self) -> StreamRow {
        StreamRow {
            id: self.id,
            device_tag: self.device_tag.clone(),
            channel_tag: self.channel_tag.clone(),
            media_server_tag: Some(self.media_server_tag.clone()),
            app: self.app.clone(),
            token: Some(self.token.clone()),
            state: self.state.to_string(),
            retry_count: Some(self.retry_count as i32),
            max_retries: Some(self.max_retries as i32),
            last_error: self.last_error.clone(),
            viewer_count: Some(self.viewer_count as i32),
            bandwidth_in: Some(self.bandwidth_in as i64),
            bandwidth_out: Some(self.bandwidth_out as i64),
            last_keepalive_at: RbatisDateTime::from_timestamp_millis(self.last_keepalive_at.timestamp_millis()),
            created_at: RbatisDateTime::from_timestamp_millis(self.created_at.timestamp_millis()),
        }
    }
}

#[sql("SELECT id, device_tag, channel_tag, media_server_tag, app, token, state, retry_count, max_retries, last_error, viewer_count, bandwidth_in, bandwidth_out, last_keepalive_at, created_at FROM streams ORDER BY created_at DESC")]
pub async fn stream_select_all(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<Vec<StreamRow>> {
    impled!()
}

#[sql("SELECT id, device_tag, channel_tag, media_server_tag, app, token, state, retry_count, max_retries, last_error, viewer_count, bandwidth_in, bandwidth_out, last_keepalive_at, created_at FROM streams WHERE id = $1")]
pub async fn stream_select_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<Vec<StreamRow>> {
    impled!()
}

#[sql("SELECT id, device_tag, channel_tag, media_server_tag, app, token, state, retry_count, max_retries, last_error, viewer_count, bandwidth_in, bandwidth_out, last_keepalive_at, created_at FROM streams WHERE token = $1")]
pub async fn stream_select_by_token(
    rb: &dyn rbatis::Executor,
    token: &str,
) -> rbatis::Result<Vec<StreamRow>> {
    impled!()
}

#[sql("INSERT INTO streams (device_tag, channel_tag, media_server_tag, app, token, state, retry_count, max_retries, last_error, viewer_count, bandwidth_in, bandwidth_out, last_keepalive_at, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id")]
pub async fn stream_insert(
    rb: &dyn rbatis::Executor,
    device_tag: Option<&str>,
    channel_tag: Option<&str>,
    media_server_tag: Option<String>,
    app: &str,
    token: Option<&str>,
    state: &str,
    retry_count: Option<i32>,
    max_retries: Option<i32>,
    last_error: Option<String>,
    viewer_count: Option<i32>,
    bandwidth_in: Option<i64>,
    bandwidth_out: Option<i64>,
    last_keepalive_at: RbatisDateTime,
    created_at: RbatisDateTime,
) -> rbatis::Result<Option<i64>> {
    impled!()
}

#[html_sql("sql/stream.html")]
pub async fn stream_update(
    rb: &dyn rbatis::Executor,
    device_tag: Option<&str>,
    channel_tag: Option<&str>,
    media_server_tag: Option<String>,
    state: Option<&str>,
    retry_count: Option<i32>,
    max_retries: Option<i32>,
    last_error: Option<String>,
    viewer_count: Option<i32>,
    bandwidth_in: Option<i64>,
    bandwidth_out: Option<i64>,
    last_keepalive_at: Option<RbatisDateTime>,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[sql("DELETE FROM streams WHERE device_tag = $1")]
pub async fn stream_delete_by_device_tag(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[sql("DELETE FROM streams WHERE id = $1")]
pub async fn stream_delete_by_id(
    rb: &dyn rbatis::Executor,
    id: i64,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

impl From<ServerRow> for crate::config::MediaServerConfig {
    fn from(row: ServerRow) -> Self {
        crate::config::MediaServerConfig {
            name: row.name,
            url: row.url,
            api_key: row.api_key,
            server_type: row.server_type,
            weight: row.weight as u32,
            enabled: row.enabled,
            server_tag: row.server_tag,
            protocol_ports: serde_json::from_value(row.protocol_ports).unwrap_or_default(),
        }
    }
}

#[sql("SELECT code, name, level, parent_code, province_name, city_name, district_name, gb28181_code, 0 as device_count
       FROM gb28181_regions WHERE level = 1 ORDER BY code")]
pub async fn region_select_provinces(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<RegionRow>> { impled!() }

#[sql("SELECT code, name, level, parent_code, province_name, city_name, district_name, gb28181_code,
              (SELECT count(*) FROM devices d WHERE d.extended->>'region_code' = r.code) as device_count
       FROM gb28181_regions r WHERE parent_code = $1 ORDER BY code")]
pub async fn region_select_children(rb: &dyn rbatis::Executor, parent_code: &str) -> rbatis::Result<Vec<RegionRow>> { impled!() }

#[sql("SELECT code, name, level, parent_code, province_name, city_name, district_name, gb28181_code,
              (SELECT count(*) FROM devices d WHERE d.extended->>'region_code' = r.code) as device_count
       FROM gb28181_regions r WHERE level <= 3 ORDER BY code")]
pub async fn region_select_all(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<RegionRow>> { impled!() }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GroupRow {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub sort_order: Option<i32>,
    pub created_at: RbatisDateTime,
    pub device_count: Option<i64>,
}

impl TryFrom<GroupRow> for crate::domain::device_group::DeviceGroupNode {
    type Error = AppError;

    fn try_from(row: GroupRow) -> Result<Self, Self::Error> {
        Ok(crate::domain::device_group::DeviceGroupNode {
            id: row.id,
            name: row.name,
            parent_id: row.parent_id,
            sort_order: row.sort_order.unwrap_or(0),
            device_count: row.device_count.unwrap_or(0) as u32,
            children: Vec::new(),
        })
    }
}

impl TryFrom<GroupRow> for crate::domain::device_group::DeviceGroup {
    type Error = AppError;

    fn try_from(row: GroupRow) -> Result<Self, Self::Error> {
        Ok(crate::domain::device_group::DeviceGroup {
            id: row.id,
            name: row.name,
            parent_id: row.parent_id,
            sort_order: row.sort_order.unwrap_or(0),
            created_at: chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
                .unwrap_or_else(chrono::Utc::now),
        })
    }
}

#[html_sql("sql/groups.html")]
pub async fn group_select_all(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<GroupRow>> { impled!() }

#[html_sql("sql/groups.html")]
pub async fn group_insert(rb: &dyn rbatis::Executor, name: &str, parent_id: Option<i64>, sort_order: i32) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[html_sql("sql/groups.html")]
pub async fn group_update(rb: &dyn rbatis::Executor, id: i64, name: &str, parent_id: Option<i64>, sort_order: i32) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[sql("DELETE FROM device_groups WHERE id = $1")]
pub async fn group_delete(rb: &dyn rbatis::Executor, id: i64) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LayoutRow {
    pub id: i64,
    pub name: String,
    pub rows: i32,
    pub cols: i32,
    pub layout_json: serde_json::Value,
    pub is_default: bool,
    pub created_at: Option<RbatisDateTime>,
    pub updated_at: Option<RbatisDateTime>,
}

impl From<LayoutRow> for crate::domain::player_layout::PlayerLayout {
    fn from(r: LayoutRow) -> Self {
        let layout_json: Vec<crate::domain::player_layout::LayoutItem> =
            serde_json::from_value(r.layout_json.clone()).unwrap_or_default();
        let created_at = r.created_at.map(|dt| chrono::DateTime::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_else(chrono::Utc::now)).unwrap_or_else(chrono::Utc::now);
        let updated_at = r.updated_at.map(|dt| chrono::DateTime::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_else(chrono::Utc::now)).unwrap_or_else(chrono::Utc::now);
        Self {
            id: r.id as i32,
            name: r.name,
            rows: r.rows,
            cols: r.cols,
            layout_json,
            is_default: r.is_default,
            created_at,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LayoutInsertResult {
    pub id: i64,
}

#[html_sql("sql/layout.html")]
pub async fn layout_select_by_id(rb: &dyn rbatis::Executor, id: i64) -> rbatis::Result<Vec<LayoutRow>> { impled!() }

#[html_sql("sql/layout.html")]
pub async fn layout_select_all(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<LayoutRow>> { impled!() }

#[html_sql("sql/layout.html")]
pub async fn layout_select_default(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<LayoutRow>> { impled!() }

#[html_sql("sql/layout.html")]
pub async fn layout_insert(rb: &dyn rbatis::Executor, name: &str, rows: i32, cols: i32, layout_json: serde_json::Value, is_default: bool) -> rbatis::Result<Vec<LayoutInsertResult>> { impled!() }

#[html_sql("sql/layout.html")]
pub async fn layout_update(rb: &dyn rbatis::Executor, id: i64, name: Option<&str>, rows: Option<i32>, cols: Option<i32>, layout_json: Option<serde_json::Value>, is_default: Option<bool>) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PtzPresetRow {
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub token: String,
    pub position_pan: Option<f64>,
    pub position_tilt: Option<f64>,
    pub position_zoom: Option<f64>,
    pub created_at: RbatisDateTime,
}

impl From<PtzPresetRow> for crate::domain::ptz::PtzPreset {
    fn from(r: PtzPresetRow) -> Self {
        Self {
            id: uuid::Uuid::parse_str(&r.id).map(|u| u.as_u64_pair().0 as i64).unwrap_or(0),
            device_id: uuid::Uuid::parse_str(&r.device_id).map(|u| u.as_u64_pair().0 as i64).unwrap_or(0),
            name: r.name,
            token: r.token,
            position_pan: r.position_pan,
            position_tilt: r.position_tilt,
            position_zoom: r.position_zoom,
            created_at: chrono::DateTime::from_timestamp(r.created_at.unix_timestamp(), 0)
                .unwrap_or_else(chrono::Utc::now),
        }
    }
}

#[sql("UPDATE ptz_presets SET name = $2 WHERE device_id = CAST($1 AS UUID) AND token = $3")]
pub async fn ptz_preset_update(
    rb: &dyn rbatis::Executor,
    device_id: String,
    name: String,
    token: String,
) -> rbatis::Result<rbatis::rbdc::ExecResult> {
    impled!()
}

#[sql("SELECT id, device_id, name, token, position_pan, position_tilt, position_zoom, created_at FROM ptz_presets WHERE device_id = CAST($1 AS UUID)")]
pub async fn ptz_preset_select_by_device_id(
    rb: &dyn rbatis::Executor,
    device_id: String,
) -> rbatis::Result<Vec<PtzPresetRow>> {
    impled!()
}

#[html_sql("sql/layout.html")]
pub async fn layout_set_default(rb: &dyn rbatis::Executor) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[sql("DELETE FROM player_window_layouts WHERE id = $1")]
pub async fn layout_delete(rb: &dyn rbatis::Executor, id: i64) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AlarmRow {
    pub id: i64,
    pub device_id: i64,
    pub device_tag: String,
    pub alarm_type: String,
    pub alarm_time: RbatisDateTime,
    pub alarm_method: i32,
    pub alarm_priority: i32,
    pub description: Option<String>,
    pub processed: bool,
    pub created_at: RbatisDateTime,
}

impl From<AlarmRow> for crate::domain::alarm::Alarm {
    fn from(row: AlarmRow) -> Self {
        let alarm_time = chrono::DateTime::from_timestamp(row.alarm_time.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);
        let created_at = chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);
        crate::domain::alarm::Alarm {
            id: row.id,
            device_id: row.device_id,
            device_tag: row.device_tag,
            alarm_type: row.alarm_type,
            alarm_time,
            alarm_method: row.alarm_method,
            alarm_priority: row.alarm_priority,
            description: row.description,
            processed: row.processed,
            created_at,
        }
    }
}

#[sql("INSERT INTO alarms (device_id, device_tag, alarm_type, alarm_time, alarm_method, alarm_priority, description) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id, device_id, device_tag, alarm_type, alarm_time, alarm_method, alarm_priority, description, processed, created_at")]
pub async fn alarm_insert(
    rb: &dyn rbatis::Executor,
    device_id: i64,
    device_tag: &str,
    alarm_type: &str,
    alarm_time: RbatisDateTime,
    alarm_method: i32,
    alarm_priority: i32,
    description: Option<&str>,
) -> rbatis::Result<Vec<AlarmRow>> { impled!() }

#[sql("SELECT id, device_id, device_tag, alarm_type, alarm_time, alarm_method, alarm_priority, description, processed, created_at FROM alarms WHERE device_id = $1 ORDER BY alarm_time DESC LIMIT $2 OFFSET $3")]
pub async fn alarm_select_by_device(
    rb: &dyn rbatis::Executor,
    device_id: i64,
    limit: i64,
    offset: i64,
) -> rbatis::Result<Vec<AlarmRow>> { impled!() }

#[sql("SELECT id, device_id, device_tag, alarm_type, alarm_time, alarm_method, alarm_priority, description, processed, created_at FROM alarms ORDER BY alarm_time DESC LIMIT $1 OFFSET $2")]
pub async fn alarm_select_all(
    rb: &dyn rbatis::Executor,
    limit: i64,
    offset: i64,
) -> rbatis::Result<Vec<AlarmRow>> { impled!() }

#[sql("SELECT COUNT(*) FROM alarms WHERE device_id = $1")]
pub async fn alarm_count_by_device(rb: &dyn rbatis::Executor, device_id: i64) -> rbatis::Result<i64> { impled!() }

#[sql("SELECT COUNT(*) FROM alarms")]
pub async fn alarm_count_all(rb: &dyn rbatis::Executor) -> rbatis::Result<i64> { impled!() }

#[sql("UPDATE alarms SET processed = $1 WHERE id = $2")]
pub async fn alarm_mark_processed(rb: &dyn rbatis::Executor, processed: bool, id: i64) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

// ============================================
// GB28181 Reference Data Queries
// ============================================

use crate::domain::gb28181_ref::{GbDeviceType, GbIndustryCode, GbNetworkCode};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GbDeviceTypeRow {
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub category: String,
    pub description: Option<String>,
    pub can_have_children: bool,
    pub sort_order: i32,
}

impl From<GbDeviceTypeRow> for GbDeviceType {
    fn from(row: GbDeviceTypeRow) -> Self {
        GbDeviceType {
            code: row.code,
            name: row.name,
            name_en: row.name_en,
            category: row.category,
            description: row.description,
            can_have_children: row.can_have_children,
            sort_order: row.sort_order,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GbIndustryCodeRow {
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub description: Option<String>,
    pub sort_order: i32,
}

impl From<GbIndustryCodeRow> for GbIndustryCode {
    fn from(row: GbIndustryCodeRow) -> Self {
        GbIndustryCode {
            code: row.code,
            name: row.name,
            name_en: row.name_en,
            description: row.description,
            sort_order: row.sort_order,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GbNetworkCodeRow {
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub description: Option<String>,
    pub sort_order: i32,
}

impl From<GbNetworkCodeRow> for GbNetworkCode {
    fn from(row: GbNetworkCodeRow) -> Self {
        GbNetworkCode {
            code: row.code,
            name: row.name,
            name_en: row.name_en,
            description: row.description,
            sort_order: row.sort_order,
        }
    }
}

#[sql("SELECT code, name, name_en, category, description, can_have_children, sort_order FROM gb_device_types ORDER BY sort_order")]
pub async fn gb_device_type_select_all(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<GbDeviceTypeRow>> { impled!() }

#[sql("SELECT code, name, name_en, description, sort_order FROM gb_industry_codes ORDER BY sort_order")]
pub async fn gb_industry_code_select_all(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<GbIndustryCodeRow>> { impled!() }

#[sql("SELECT code, name, name_en, description, sort_order FROM gb_network_codes ORDER BY sort_order")]
pub async fn gb_network_code_select_all(rb: &dyn rbatis::Executor) -> rbatis::Result<Vec<GbNetworkCodeRow>> { impled!() }

impl TryFrom<ChannelRow> for Channel {
    type Error = AppError;

    fn try_from(row: ChannelRow) -> Result<Self, Self::Error> {
        let status = match row.status.to_lowercase().as_str() {
            "online" => DeviceStatus::Online,
            "error" => DeviceStatus::Error,
            "maintaining" => DeviceStatus::Maintaining,
            _ => DeviceStatus::Offline,
        };

        let device_type = row.device_type.as_ref()
            .map(|dt| match dt.as_str() {
                "NVR" => DeviceType::NVR,
                "IPC" => DeviceType::IPC,
                "DVR" => DeviceType::DVR,
                "Encoder" => DeviceType::Encoder,
                "VideoServer" => DeviceType::VideoServer,
                "Camera" => DeviceType::Camera,
                "Platform" => DeviceType::Platform,
                _ => DeviceType::Other,
            })
            .unwrap_or(DeviceType::Other);

        let created_at = chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);

        let updated_at = row.updated_at.and_then(|u| chrono::DateTime::from_timestamp(u.unix_timestamp(), 0));

        Ok(Channel {
            id: row.id,
            device_tag: row.device_tag,
            channel_tag: row.channel_tag,
            name: row.name,
            status,
            device_type,
            device_type_code: row.device_type_code,
            channel_extended: row.channel_extended,
            is_default: row.is_default.unwrap_or(false),
            parent_channel_tag: row.parent_channel_tag,
            civil_code: row.civil_code,
            address: row.address,
            ip_address: row.ip_address,
            port: row.port.unwrap_or(0) as u16,
            manufacturer: row.manufacturer,
            model: row.model,
            parental: row.parental.unwrap_or(0),
            extended: row.extended,
            created_at,
            updated_at,
        })
    }
}

#[html_sql("sql/channel.html")]
pub async fn channel_select_by_device_tag(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
) -> rbatis::Result<Vec<ChannelRow>> { impled!() }

#[html_sql("sql/channel.html")]
pub async fn channel_select_by_device_tag_and_channel_tag(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
    channel_tag: &str,
) -> rbatis::Result<Vec<ChannelRow>> { impled!() }

#[html_sql("sql/channel.html")]
pub async fn channel_select_all(
    rb: &dyn rbatis::Executor,
) -> rbatis::Result<Vec<ChannelRow>> { impled!() }

#[html_sql("sql/channel.html")]
pub async fn channel_insert(
    rb: &dyn rbatis::Executor,
    device_tag: Option<&str>,
    channel_tag: Option<&str>,
    name: Option<&str>,
    status: Option<&str>,
    device_type: Option<&str>,
    device_type_code: Option<&str>,
    channel_extended: Option<serde_json::Value>,
    is_default: Option<bool>,
    parent_channel_tag: Option<&str>,
    civil_code: Option<&str>,
    address: Option<&str>,
    ip_address: Option<&str>,
    port: Option<i32>,
    manufacturer: Option<&str>,
    model: Option<&str>,
    parental: Option<i32>,
    extended: Option<serde_json::Value>,
    created_at: Option<RbatisDateTime>,
    updated_at: Option<RbatisDateTime>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[html_sql("sql/channel.html")]
pub async fn channel_update(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
    channel_tag: &str,
    name: Option<&str>,
    status: Option<&str>,
    device_type: Option<&str>,
    device_type_code: Option<&str>,
    channel_extended: Option<serde_json::Value>,
    is_default: Option<bool>,
    parent_channel_tag: Option<&str>,
    civil_code: Option<&str>,
    address: Option<&str>,
    ip_address: Option<&str>,
    port: Option<i32>,
    manufacturer: Option<&str>,
    model: Option<&str>,
    parental: Option<i32>,
    extended: Option<serde_json::Value>,
    updated_at: Option<RbatisDateTime>,
) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }

#[html_sql("sql/channel.html")]
pub async fn channel_delete_by_device_tag(
    rb: &dyn rbatis::Executor,
    device_tag: &str,
) -> rbatis::Result<rbatis::rbdc::ExecResult> { impled!() }
