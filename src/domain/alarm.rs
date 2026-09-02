use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    pub id: i64,
    pub device_id: i64,
    pub device_tag: String,
    pub alarm_type: String,
    pub alarm_time: DateTime<Utc>,
    pub alarm_method: i32,
    pub alarm_priority: i32,
    pub description: Option<String>,
    pub processed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlarmRequest {
    pub device_id: i64,
    pub device_tag: String,
    pub alarm_type: String,
    pub alarm_time: DateTime<Utc>,
    pub alarm_method: Option<i32>,
    pub alarm_priority: Option<i32>,
    pub description: Option<String>,
}
