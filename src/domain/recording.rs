use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RecordingState {
    Starting,
    Recording,
    Paused,
    Stopping,
    Completed,
    Error,
}

impl Default for RecordingState {
    fn default() -> Self {
        RecordingState::Starting
    }
}

impl fmt::Display for RecordingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordingState::Starting => write!(f, "starting"),
            RecordingState::Recording => write!(f, "recording"),
            RecordingState::Paused => write!(f, "paused"),
            RecordingState::Stopping => write!(f, "stopping"),
            RecordingState::Completed => write!(f, "completed"),
            RecordingState::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RecordingFormat {
    Hls,
    Mp4,
    Flv,
    Ts,
}

impl fmt::Display for RecordingFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordingFormat::Hls => write!(f, "HLS"),
            RecordingFormat::Mp4 => write!(f, "MP4"),
            RecordingFormat::Flv => write!(f, "FLV"),
            RecordingFormat::Ts => write!(f, "TS"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: i64,
    pub device_tag: Option<String>,
    pub channel_tag: Option<String>,
    pub media_server_name: String,
    pub state: RecordingState,
    pub format: RecordingFormat,
    pub output_path: Option<String>,
    pub file_size: u64,
    pub duration_secs: u64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub labels: Vec<String>,
    pub filename: Option<String>,
}

impl Recording {
    pub fn new(
        device_tag: String,
        channel_tag: String,
        media_server_name: String,
        format: RecordingFormat,
    ) -> Self {
        Self {
            id: 0,
            device_tag: Some(device_tag),
            channel_tag: Some(channel_tag),
            media_server_name,
            state: RecordingState::Starting,
            format,
            output_path: None,
            file_size: 0,
            duration_secs: 0,
            created_at: Utc::now(),
            started_at: None,
            stopped_at: None,
            error_message: None,
            labels: Vec::new(),
            filename: None,
        }
    }

    pub fn stream_key(&self) -> String {
        let device = self.device_tag.as_deref().unwrap_or("");
        let channel = self.channel_tag.as_deref().unwrap_or("");
        crate::domain::stream::make_stream_key(device, channel)
    }

    pub fn start(&mut self) {
        self.state = RecordingState::Recording;
        self.started_at = Some(Utc::now());
    }

    pub fn pause(&mut self) {
        if self.state == RecordingState::Recording {
            self.state = RecordingState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == RecordingState::Paused {
            self.state = RecordingState::Recording;
        }
    }

    pub fn stop(&mut self) {
        self.state = RecordingState::Stopping;
        self.stopped_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.state = RecordingState::Completed;
        self.stopped_at = Some(Utc::now());
        if let (Some(start), Some(stop)) = (self.started_at, self.stopped_at) {
            self.duration_secs = (stop - start).num_seconds() as u64;
        }
    }

    pub fn set_error(&mut self, message: String) {
        self.state = RecordingState::Error;
        self.error_message = Some(message);
    }

    pub fn set_output(&mut self, path: String, file_size: u64) {
        self.output_path = Some(path);
        self.file_size = file_size;
    }

    pub fn file_size_mb(&self) -> f64 {
        self.file_size as f64 / (1024.0 * 1024.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecordingRequest {
    pub device_tag: String,
    pub channel_tag: String,
    pub format: Option<RecordingFormat>,
    pub duration_secs: Option<u32>,
    pub max_file_size_mb: Option<u32>,
    pub output_path: Option<String>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingListQuery {
    pub device_tag: Option<String>,
    pub state: Option<RecordingState>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
