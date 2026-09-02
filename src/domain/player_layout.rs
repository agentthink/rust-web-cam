use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutItem {
    pub id: String,
    pub row: usize,
    pub col: usize,
    #[serde(default = "default_one")]
    pub row_span: usize,
    #[serde(default = "default_one")]
    pub col_span: usize,
    #[serde(default)]
    pub label: Option<String>,
}

fn default_one() -> usize { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerLayout {
    pub id: i32,
    pub name: String,
    pub rows: i32,
    pub cols: i32,
    pub layout_json: Vec<LayoutItem>,
    #[serde(default)]
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CreateLayoutRequest {
    pub name: String,
    #[serde(default)]
    pub rows: i32,
    #[serde(default)]
    pub cols: i32,
    pub layout_json: Vec<LayoutItem>,
    #[serde(default)]
    pub is_default: bool,
}

impl Default for CreateLayoutRequest {
    fn default() -> Self {
        Self {
            name: String::new(),
            rows: 2,
            cols: 2,
            layout_json: Vec::new(),
            is_default: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UpdateLayoutRequest {
    pub name: Option<String>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub layout_json: Option<Vec<LayoutItem>>,
    pub is_default: Option<bool>,
}

impl Default for UpdateLayoutRequest {
    fn default() -> Self {
        Self {
            name: None,
            rows: None,
            cols: None,
            layout_json: None,
            is_default: None,
        }
    }
}