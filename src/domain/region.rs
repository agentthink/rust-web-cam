use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub code: String,
    pub name: String,
    pub level: i16,
    pub parent_code: Option<String>,
    pub province_name: Option<String>,
    pub city_name: Option<String>,
    pub district_name: Option<String>,
    pub gb28181_code: String,
    #[serde(default)]
    pub device_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionNode {
    pub code: String,
    pub name: String,
    pub level: i16,
    pub gb28181_code: String,
    #[serde(default)]
    pub parent_code: Option<String>,
    pub children: Vec<RegionNode>,
    pub device_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegionRow {
    pub code: String,
    pub name: String,
    pub level: Option<i16>,
    pub parent_code: Option<String>,
    pub province_name: Option<String>,
    pub city_name: Option<String>,
    pub district_name: Option<String>,
    pub gb28181_code: Option<String>,
    pub device_count: Option<i64>,
}

impl RegionRow {
    pub fn to_region(&self) -> Region {
        Region {
            code: self.code.clone(),
            name: self.name.clone(),
            level: self.level.unwrap_or(0),
            parent_code: self.parent_code.clone(),
            province_name: self.province_name.clone(),
            city_name: self.city_name.clone(),
            district_name: self.district_name.clone(),
            gb28181_code: self.gb28181_code.clone().unwrap_or_default(),
            device_count: self.device_count.unwrap_or(0) as u32,
        }
    }

    pub fn to_node(&self) -> RegionNode {
        RegionNode {
            code: self.code.clone(),
            name: self.name.clone(),
            level: self.level.unwrap_or(0),
            gb28181_code: self.gb28181_code.clone().unwrap_or_default(),
            parent_code: self.parent_code.clone(),
            children: Vec::new(),
            device_count: self.device_count.unwrap_or(0) as u32,
        }
    }
}