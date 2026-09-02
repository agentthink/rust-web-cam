use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GbDeviceType {
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub category: String,
    pub description: Option<String>,
    pub can_have_children: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GbIndustryCode {
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GbNetworkCode {
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GbReferenceData {
    pub device_types: Vec<GbDeviceType>,
    pub industry_codes: Vec<GbIndustryCode>,
    pub network_codes: Vec<GbNetworkCode>,
}
