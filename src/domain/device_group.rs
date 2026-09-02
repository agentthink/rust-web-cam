use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGroup {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub parent_id: Option<i64>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub parent_id: Option<i64>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGroupNode {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
    pub device_count: u32,
    pub children: Vec<DeviceGroupNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignGroupRequest {
    pub group_id: Option<i64>,
}