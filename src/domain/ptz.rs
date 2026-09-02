use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzPreset {
    pub id: i64,
    pub device_id: i64,
    pub name: String,
    pub token: String,
    pub position_pan: Option<f64>,
    pub position_tilt: Option<f64>,
    pub position_zoom: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzStatus {
    pub position_pan: Option<f64>,
    pub position_tilt: Option<f64>,
    pub position_zoom: Option<f64>,
    pub moving: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzControlLog {
    pub id: i64,
    pub user_id: Option<i64>,
    pub device_id: i64,
    pub command: String,
    pub speed: u8,
    pub result: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtzMoveType {
    Continuous,
    Absolute,
    Relative,
    GotoPreset,
    SetPreset,
    RemovePreset,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzControlRequest {
    pub move_type: Option<PtzMoveType>,
    pub command: Option<String>,
    pub speed: Option<u8>,
    pub pan: Option<f64>,
    pub tilt: Option<f64>,
    pub zoom: Option<f64>,
    pub preset_token: Option<String>,
    pub preset_name: Option<String>,
    pub channel: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzControlResponse {
    pub success: bool,
    pub message: Option<String>,
    pub preset_token: Option<String>,
    pub status: Option<PtzStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzControlResult {
    pub success: bool,
    pub message: String,
    pub preset_token: Option<String>,
}

impl PtzPreset {
    pub fn new(device_id: i64, name: String, token: String) -> Self {
        Self {
            id: 0,
            device_id,
            name,
            token,
            position_pan: None,
            position_tilt: None,
            position_zoom: None,
            created_at: Utc::now(),
        }
    }
}