use crate::domain::Protocol;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    Play,
    Record,
    Playback,
    Ptz,
}

impl fmt::Display for SessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionType::Play => write!(f, "play"),
            SessionType::Record => write!(f, "record"),
            SessionType::Playback => write!(f, "playback"),
            SessionType::Ptz => write!(f, "ptz"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SessionState {
    Initializing,
    Active,
    Idle,
    Terminating,
    Terminated,
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionState::Initializing => write!(f, "initializing"),
            SessionState::Active => write!(f, "active"),
            SessionState::Idle => write!(f, "idle"),
            SessionState::Terminating => write!(f, "terminating"),
            SessionState::Terminated => write!(f, "terminated"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub session_type: SessionType,
    pub device_tag: Option<String>,
    pub channel_tag: Option<String>,
    pub user_id: i64,
    pub state: SessionState,

    pub client_ip: Option<String>,
    pub client_type: Option<String>,

    pub media_server_tag: Option<String>,
    pub protocol: Option<Protocol>,

    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,

    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl Session {
    pub fn new(session_type: SessionType, user_id: i64) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            session_type,
            device_tag: None,
            channel_tag: None,
            user_id,
            state: SessionState::Initializing,
            client_ip: None,
            client_type: None,
            media_server_tag: None,
            protocol: None,
            created_at: now,
            last_activity: now,
            expires_at: None,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    pub fn stream_key(&self) -> String {
        match (&self.device_tag, &self.channel_tag) {
            (Some(dt), Some(ct)) => format!("{}_{}", dt, ct),
            (Some(dt), None) => dt.clone(),
            _ => String::new(),
        }
    }

    pub fn set_active(&mut self, media_server_tag: String) {
        self.state = SessionState::Active;
        self.media_server_tag = Some(media_server_tag);
        self.last_activity = Utc::now();
    }

    pub fn set_idle(&mut self) {
        self.state = SessionState::Idle;
        self.last_activity = Utc::now();
    }

    pub fn terminate(&mut self) {
        self.state = SessionState::Terminating;
        self.last_activity = Utc::now();
    }

    pub fn mark_terminated(&mut self) {
        self.state = SessionState::Terminated;
        self.last_activity = Utc::now();
    }

    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    pub fn refresh_expiry(&mut self, duration_secs: i64) {
        self.expires_at = Some(Utc::now() + chrono::TimeDelta::seconds(duration_secs));
        self.last_activity = Utc::now();
    }

    pub fn update_stats(&mut self, bytes_sent: u64, bytes_received: u64) {
        self.bytes_sent = bytes_sent;
        self.bytes_received = bytes_received;
        self.last_activity = Utc::now();
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            return Utc::now() > expires_at;
        }
        false
    }
}
