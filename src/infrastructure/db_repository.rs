use std::sync::Arc;
use dashmap::DashMap;
use rbatis::rbdc::datetime::DateTime as RbatisDateTime;
use rbatis::RBatis;

use crate::domain::server::Server;
use crate::domain::{Alarm, CreateAlarmRequest, Channel, Device, DeviceStatus, Protocol, Recording, Session, Stream};
use crate::domain::device_group::{CreateGroupRequest, DeviceGroup, DeviceGroupNode, UpdateGroupRequest};
use crate::domain::ptz::PtzPreset;
use crate::domain::region::{Region, RegionNode};
use crate::domain::player_layout::PlayerLayout;
use crate::error::AppError;
use crate::error::Result;
use crate::sql_mappers::{
    device_select_all, server_select_all, session_select_all, stream_select_all,
    recording_select_all, recording_select_paginated, recording_count_all, recording_select_by_id, recording_select_by_device_tag, device_insert, device_update, device_delete_by_id,
    device_delete_by_device_tag, device_select_by_parent_device_tag,
    group_select_all, group_insert, group_update, group_delete,
    region_select_all, region_select_children,
    server_insert, server_update, server_delete_by_id, server_select_by_tag,
    session_insert, session_delete_by_id,
    stream_insert, stream_update, stream_select_by_token, stream_delete_by_device_tag,
    recording_insert, recording_update, recording_delete_by_id,
    alarm_insert, alarm_select_by_device, alarm_select_all,
    alarm_count_by_device, alarm_count_all, alarm_mark_processed,
    channel_insert, channel_update, channel_select_all,
    channel_select_by_device_tag, channel_select_by_device_tag_and_channel_tag,
    ServerRow, GroupRow, SessionRow,
    layout_select_by_id, layout_select_all, layout_select_default,
    layout_insert, layout_update, layout_set_default, layout_delete,
};
use rbatis::RBatisRef;

pub struct DbRepository {
    rb: RBatis,
    pub devices_cache: Arc<DashMap<String, Device>>,
    /// 主缓存：key = "device_tag/channel_tag"
    pub channels_cache: Arc<DashMap<String, Channel>>,
    pub servers_cache: Arc<DashMap<i64, ServerRow>>,
    pub sessions_cache: Arc<DashMap<i64, Session>>,
    /// 主缓存：key = "device_tag/channel_tag"
    pub streams_cache: Arc<DashMap<String, Stream>>,
}

fn stream_key_to_cache_key(stream_key: &str) -> String {
    stream_key.replace('_', "/")
}

fn cache_key_to_stream_key(cache_key: &str) -> String {
    cache_key.replace('/', "_")
}

fn format_cache_key(device_tag: Option<&str>, channel_tag: Option<&str>) -> String {
    format!(
        "{}/{}",
        device_tag.unwrap_or_default(),
        channel_tag.unwrap_or_default()
    )
}

impl DbRepository {
    pub async fn new(url: &str, debug_sql: bool) -> anyhow::Result<Self> {
        let rb = RBatis::new();
        rb.init(rbdc_pg::driver::PgDriver {}, url).ok();
        if debug_sql {
            tracing::info!("[DB] debug_sql enabled - SQL will be logged at INFO level");
        }
        let repo = Self {
            rb,
            devices_cache: Arc::new(DashMap::new()),
            channels_cache: Arc::new(DashMap::new()),
            servers_cache: Arc::new(DashMap::new()),
            sessions_cache: Arc::new(DashMap::new()),
            streams_cache: Arc::new(DashMap::new()),
        };
        repo.load_caches().await?;
        Ok(repo)
    }

    pub async fn new_without_load(url: &str, debug_sql: bool) -> anyhow::Result<Self> {
        let rb = RBatis::new();
        rb.init(rbdc_pg::driver::PgDriver {}, url).ok();
        if debug_sql {
            tracing::info!("[DB] debug_sql enabled - SQL will be logged at INFO for SQL");
        }
        let repo = Self {
            rb,
            devices_cache: Arc::new(DashMap::new()),
            channels_cache: Arc::new(DashMap::new()),
            servers_cache: Arc::new(DashMap::new()),
            sessions_cache: Arc::new(DashMap::new()),
            streams_cache: Arc::new(DashMap::new()),
        };
        Ok(repo)
    }

    pub async fn load_caches(&self) -> anyhow::Result<()> {
        let devices = device_select_all(self.rb.acquire().await?.rb_ref())
            .await
            .map_err(|e| anyhow::anyhow!("load devices: {}", e))?;
        for row in devices {
            if let Ok(device) = Device::try_from(row) {
                if let Some(ref tag) = device.device_tag {
                    self.devices_cache.insert(tag.clone(), device.clone());
                }
            }
        }

        let channels = channel_select_all(self.rb.acquire().await?.rb_ref())
            .await
            .map_err(|e| anyhow::anyhow!("load channels: {}", e))?;
        for row in channels {
            if let Ok(channel) = Channel::try_from(row) {
                let key = format_cache_key(Some(&channel.device_tag), Some(&channel.channel_tag));
                self.channels_cache.insert(key, channel);
            }
        }

        let servers = server_select_all(self.rb.acquire().await?.rb_ref())
            .await
            .map_err(|e| anyhow::anyhow!("load servers: {}", e))?;
        for server in servers {
            self.servers_cache.insert(server.id, server);
        }

        let sessions = session_select_all(self.rb.acquire().await?.rb_ref())
            .await
            .map_err(|e| anyhow::anyhow!("load sessions: {}", e))?;
        for row in sessions {
            if let Ok(session) = Session::try_from(row) {
                self.sessions_cache.insert(session.id, session);
            }
        }

        let streams = stream_select_all(self.rb.acquire().await?.rb_ref())
            .await
            .map_err(|e| anyhow::anyhow!("load streams: {}", e))?;
        for row in streams {
            if let Ok(stream) = Stream::try_from(row) {
                let key = format_cache_key(stream.device_tag.as_deref(), stream.channel_tag.as_deref());
                self.streams_cache.insert(key, stream);
            }
        }

        tracing::info!(
            "[DB] Caches loaded: {} devices, {} channels, {} servers, {} sessions, {} streams",
            self.devices_cache.len(),
            self.channels_cache.len(),
            self.servers_cache.len(),
            self.sessions_cache.len(),
            self.streams_cache.len(),
        );
        Ok(())
    }

    pub async fn reload_devices_cache(&self) -> anyhow::Result<usize> {
        let devices = device_select_all(self.rb.acquire().await?.rb_ref())
            .await
            .map_err(|e| anyhow::anyhow!("reload devices: {}", e))?;
        let old_len = self.devices_cache.len();
        self.devices_cache.clear();
        let mut count = 0;
        for row in devices {
            if let Ok(device) = Device::try_from(row) {
                if let Some(ref tag) = device.device_tag {
                    self.devices_cache.insert(tag.clone(), device);
                    count += 1;
                }
            }
        }
        tracing::info!("[DB] Devices cache reloaded: {} -> {}", old_len, count);
        Ok(count)
    }

    pub fn rb(&self) -> &RBatis {
        &self.rb
    }

    // ─── Device methods ───────────────────────────────────────────────

    pub fn devices_cache(&self) -> &Arc<DashMap<String, Device>> {
        &self.devices_cache
    }

    pub fn channels_cache(&self) -> &Arc<DashMap<String, Channel>> {
        &self.channels_cache
    }

    pub async fn create_device(&self, device: &Device) -> Result<i64> {
        // 从 extended 中提取协议特有字段
        let rtsp_url = device.extended.as_ref()
            .and_then(|e| e.get("rtsp_full_url"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let gb_username = device.extended.as_ref()
            .and_then(|e| e.get("gb_username"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let gb_password = device.extended.as_ref()
            .and_then(|e| e.get("gb_password"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let ssrc = device.extended.as_ref()
            .and_then(|e| e.get("ssrc"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut extended_for_db = device.extended.clone().unwrap_or(serde_json::Value::Object(Default::default()));
        if let Some(stream_config) = &device.stream_config {
            if let Some(obj) = extended_for_db.as_object_mut() {
                obj.insert("stream_config".to_string(), serde_json::json!(stream_config));
            }
        }

        let device_type_str = device.device_type.to_string();

        let result = device_insert(
            self.rb.acquire().await?.rb_ref(),
            Some(&device.name),
            Some(&protocol_to_str(&device.protocol)),
            Some(&status_to_str(&device.status)),
            Some(&device.host),
            Some(device.port as i32),
            rtsp_url.as_deref(),
            device.device_tag.as_deref(),
            device.parent_device_tag.as_deref(),
            Some(device.is_public),
            Some(RbatisDateTime::from_timestamp_millis(device.created_at.timestamp_millis())),
            device.device_password.as_deref(),
            device.playback_username.as_deref(),
            device.playback_password.as_deref(),
            device.media_server_tag.as_deref(),
            serde_json::to_value(&device.push_urls).ok(),
            serde_json::to_value(&device.pull_urls).ok(),
            device.region_code.as_deref(),
            Some(RbatisDateTime::from_timestamp_millis(chrono::Utc::now().timestamp_millis())),
            ssrc.as_deref(),
            Some(extended_for_db.clone()),
            device.group_id,
            device.app.as_deref(),
            Some(&device_type_str),
            device.device_type_code.as_deref(),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;

        self.devices_cache.insert(device.device_tag.clone().unwrap_or_default(), device.clone());
        Ok(result.last_insert_id.as_i64().unwrap_or(0))
    }

    pub async fn get_device(&self, device_tag: &str) -> Result<Option<Device>> {
        Ok(self.devices_cache.get(device_tag).map(|r| r.clone()))
    }

    pub async fn get_device_by_id(&self, id: i64) -> Result<Option<Device>> {
        let rows: Vec<crate::sql_mappers::DeviceRow> = crate::sql_mappers::device_select_by_id(self.rb.acquire().await?.rb_ref(), id).await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(rows.into_iter().next().and_then(|r| Device::try_from(r).ok()))
    }

    pub fn get_device_by_device_tag(&self, device_tag: &str) -> Option<Device> {
        self.devices_cache.get(device_tag).map(|r| r.clone())
    }

    pub fn get_device_by_stream_key(&self, stream_key: &str) -> Option<Device> {
        let cache_key = self.get_stream_cache_key_by_stream_key(stream_key)?;
        let stream = self.streams_cache.get(&cache_key)?;
        let device_tag = stream.device_tag.as_deref()?;
        self.get_device_by_device_tag(device_tag)
    }

    pub async fn get_device_by_protocol_and_host(&self, protocol: &Protocol, host: &str) -> Option<Device> {
        self.devices_cache.iter()
            .find(|d| d.protocol == *protocol && d.host == host)
            .map(|r| r.clone())
    }

    pub async fn list_devices(&self) -> Vec<Device> {
        self.devices_cache.iter()
            .map(|d| {
                let mut device = d.clone();
                if let Some(ref tag) = device.device_tag {
                    device.has_stream = self.device_has_active_stream(tag);
                }
                device
            })
            .collect()
    }

    pub async fn list_devices_paginated(&self, limit: usize, offset: usize, search: Option<&str>) -> Vec<Device> {
        let search_lower = search.map(|s| s.to_lowercase());
        self.devices_cache.iter()
            .filter(|d| {
                if let Some(ref q) = search_lower {
                    d.name.to_lowercase().contains(q)
                        || d.host.to_lowercase().contains(q)
                        || d.device_tag.as_ref().map(|t| t.to_lowercase().contains(q)).unwrap_or(false)
                } else {
                    true
                }
            })
            .skip(offset)
            .take(limit)
            .map(|d| {
                let mut device = d.clone();
                if let Some(ref tag) = device.device_tag {
                    device.has_stream = self.device_has_active_stream(tag);
                }
                device
            })
            .collect()
    }

    pub fn device_has_active_stream(&self, device_tag: &str) -> bool {
        self.streams_cache.iter()
            .any(|s| s.device_tag.as_deref() == Some(device_tag) && s.state.is_alive())
    }

    pub async fn count_devices_filtered(&self, search: Option<&str>) -> usize {
        let search_lower = search.map(|s| s.to_lowercase());
        self.devices_cache.iter()
            .filter(|d| {
                if let Some(ref q) = search_lower {
                    d.name.to_lowercase().contains(q)
                        || d.host.to_lowercase().contains(q)
                        || d.device_tag.as_ref().map(|t| t.to_lowercase().contains(q)).unwrap_or(false)
                } else {
                    true
                }
            })
            .count()
    }

    pub async fn count_devices(&self) -> usize {
        self.devices_cache.len()
    }

    pub async fn count_devices_top_level(&self) -> usize {
        self.devices_cache.iter()
            .filter(|d| d.parent_device_tag.is_none())
            .count()
    }

    pub async fn list_online_devices_paginated(&self, limit: usize, offset: usize) -> Vec<Device> {
        self.devices_cache.iter()
            .filter(|d| d.status == DeviceStatus::Online)
            .skip(offset)
            .take(limit)
            .map(|r| r.clone())
            .collect()
    }

    pub async fn count_online_devices(&self) -> usize {
        self.devices_cache.iter()
            .filter(|d| d.status == DeviceStatus::Online)
            .count()
    }

    pub async fn update_device(&self, device: &Device) -> Result<()> {
        let mut extended_json = device.extended.clone().unwrap_or(serde_json::Value::Object(Default::default()));
        if let Some(stream_config) = &device.stream_config {
            if let Some(obj) = extended_json.as_object_mut() {
                obj.insert("stream_config".to_string(), serde_json::json!(stream_config));
            }
        }
        let push_urls_json = serde_json::to_value(&device.push_urls).ok();
        let pull_urls_json = serde_json::to_value(&device.pull_urls).ok();

        // 从 extended 中提取协议特有字段
        let rtsp_url = extended_json.get("rtsp_full_url")
            .and_then(|v| v.as_str())
            .map(String::from);
        let gb_username = extended_json.get("gb_username")
            .and_then(|v| v.as_str())
            .map(String::from);
        let gb_password = extended_json.get("gb_password")
            .and_then(|v| v.as_str())
            .map(String::from);
        let ssrc = extended_json.get("ssrc")
            .and_then(|v| v.as_str())
            .map(String::from);

        let device_type_str = device.device_type.to_string();

        device_update(
            self.rb.acquire().await?.rb_ref(),
            device.id,
            Some(&device.name),
            Some(&protocol_to_str(&device.protocol)),
            Some(&status_to_str(&device.status)),
            Some(&device.host),
            Some(device.port as i32),
            rtsp_url.as_deref(),
            device.device_tag.as_deref(),
            device.parent_device_tag.as_deref(),
            Some(device.is_public),
            device.device_password.as_deref(),
            device.playback_username.as_deref(),
            device.playback_password.as_deref(),
            device.media_server_tag.as_deref(),
            push_urls_json,
            pull_urls_json,
            device.region_code.as_deref(),
            Some(RbatisDateTime::from_timestamp_millis(chrono::Utc::now().timestamp_millis())),
            ssrc.as_deref(),
            gb_username.as_deref(),
            gb_password.as_deref(),
            Some(extended_json),
            device.group_id,
            device.app.as_deref(),
            Some(&device_type_str),
            device.device_type_code.as_deref(),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;

        self.devices_cache.insert(device.device_tag.clone().unwrap_or_default(), device.clone());
        Ok(())
    }

    pub async fn delete_device(&self, device_tag: &str) -> Result<()> {
        device_delete_by_device_tag(self.rb.acquire().await?.rb_ref(), device_tag)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        self.devices_cache.remove(device_tag);
        Ok(())
    }

    pub async fn get_children_by_parent_tag(&self, parent_tag: &str) -> Result<Vec<Device>> {
        let rows = device_select_by_parent_device_tag(self.rb.acquire().await?.rb_ref(), parent_tag)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(rows.into_iter().filter_map(|r| Device::try_from(r).ok()).collect())
    }

    pub async fn update_device_group(&self, device_id: i64, group_id: Option<i64>) -> Result<()> {
        if let Some(mut device) = self.get_device_by_id(device_id).await? {
            device.group_id = group_id;
            self.update_device(&device).await?;
        }
        Ok(())
    }

    // ─── Channel methods ────────────────────────────────────────────────

    pub async fn create_channel(&self, channel: &Channel) -> Result<i64> {
        let status_str = channel.status.to_string();
        let device_type_str = channel.device_type.to_string();
        let channel_extended_json = channel.channel_extended.clone();
        let extended_json = channel.extended.clone().unwrap_or(serde_json::Value::Object(Default::default()));

        let result = channel_insert(
            self.rb.acquire().await?.rb_ref(),
            Some(&channel.device_tag),
            Some(&channel.channel_tag),
            Some(&channel.name),
            Some(&status_str),
            Some(&device_type_str),
            channel.device_type_code.as_deref(),
            channel_extended_json,
            Some(channel.is_default),
            channel.parent_channel_tag.as_deref(),
            channel.civil_code.as_deref(),
            channel.address.as_deref(),
            channel.ip_address.as_deref(),
            Some(channel.port as i32),
            channel.manufacturer.as_deref(),
            channel.model.as_deref(),
            Some(channel.parental),
            Some(extended_json),
            Some(RbatisDateTime::from_timestamp_millis(channel.created_at.timestamp_millis())),
            channel.updated_at.map(|u| RbatisDateTime::from_timestamp_millis(u.timestamp_millis())),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let id = result.last_insert_id.as_i64().unwrap_or(0);
        let key = format_cache_key(Some(&channel.device_tag), Some(&channel.channel_tag));
        self.channels_cache.insert(key, channel.clone());
        Ok(id)
    }

    pub async fn update_channel(&self, channel: &Channel) -> Result<()> {
        let status_str = channel.status.to_string();
        let device_type_str = channel.device_type.to_string();
        let channel_extended_json = channel.channel_extended.clone();
        let extended_json = channel.extended.clone().unwrap_or(serde_json::Value::Object(Default::default()));

        channel_update(
            self.rb.acquire().await?.rb_ref(),
            &channel.device_tag,
            &channel.channel_tag,
            Some(&channel.name),
            Some(&status_str),
            Some(&device_type_str),
            channel.device_type_code.as_deref(),
            channel_extended_json,
            Some(channel.is_default),
            channel.parent_channel_tag.as_deref(),
            channel.civil_code.as_deref(),
            channel.address.as_deref(),
            channel.ip_address.as_deref(),
            Some(channel.port as i32),
            channel.manufacturer.as_deref(),
            channel.model.as_deref(),
            Some(channel.parental),
            Some(extended_json),
            Some(RbatisDateTime::from_timestamp_millis(chrono::Utc::now().timestamp_millis())),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let key = format_cache_key(Some(&channel.device_tag), Some(&channel.channel_tag));
        self.channels_cache.insert(key, channel.clone());

        Ok(())
    }

    pub async fn get_channels_by_device_tag(&self, device_tag: &str) -> Result<Vec<Channel>> {
        let rows = channel_select_by_device_tag(self.rb.acquire().await?.rb_ref(), device_tag)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(rows.into_iter().filter_map(|r| Channel::try_from(r).ok()).collect())
    }

    pub async fn get_channel(&self, device_tag: &str, channel_tag: &str) -> Result<Option<Channel>> {
        tracing::debug!("[DB] get_channel: device_tag={}, channel_tag={}", device_tag, channel_tag);
        let rows = channel_select_by_device_tag_and_channel_tag(
            self.rb.acquire().await?.rb_ref(),
            device_tag,
            channel_tag,
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        tracing::debug!("[DB] get_channel result: {} rows", rows.len());
        Ok(rows.into_iter().filter_map(|r| Channel::try_from(r).ok()).next())
    }

    pub fn get_channels_by_device_tag_cached(&self, device_tag: &str) -> Vec<Channel> {
        self.channels_cache.iter()
            .filter(|c| c.device_tag == device_tag)
            .map(|r| r.clone())
            .collect()
    }

    pub fn list_all_channels_cached(&self) -> Vec<Channel> {
        self.channels_cache.iter().map(|r| r.clone()).collect()
    }

    pub async fn list_all_channels(&self) -> Result<Vec<Channel>> {
        let rows = channel_select_all(self.rb.acquire().await?.rb_ref())
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(rows.into_iter().filter_map(|r| Channel::try_from(r).ok()).collect())
    }

    // ─── Group methods ────────────────────────────────────────────────

    pub async fn list_groups(&self) -> Vec<DeviceGroup> {
        let conn = match self.rb.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("list_groups acquire: {}", e);
                return Vec::new();
            }
        };
        match group_select_all(conn.rb_ref()).await {
            Ok(rows) => rows.into_iter().filter_map(|r| DeviceGroup::try_from(r).ok()).collect(),
            Err(e) => {
                tracing::error!("list_groups: {}", e);
                Vec::new()
            }
        }
    }

    pub async fn create_group(&self, req: &CreateGroupRequest) -> Result<i64> {
        let result = group_insert(
            self.rb.acquire().await?.rb_ref(),
            &req.name,
            req.parent_id,
            req.sort_order.unwrap_or(0) as i32,
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(result.last_insert_id.as_i64().unwrap_or(0))
    }

    pub async fn update_group(&self, id: i64, req: &UpdateGroupRequest) -> Result<()> {
        group_update(
            self.rb.acquire().await?.rb_ref(),
            id,
            req.name.as_deref().unwrap_or(""),
            req.parent_id,
            req.sort_order.unwrap_or(0) as i32,
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_group(&self, id: i64) -> Result<()> {
        group_delete(self.rb.acquire().await?.rb_ref(), id)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn get_group_tree(&self) -> Vec<DeviceGroupNode> {
        let conn = match self.rb.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("get_group_tree acquire: {}", e);
                return Vec::new();
            }
        };
        match group_select_all(conn.rb_ref()).await {
            Ok(rows) => build_group_tree(rows),
            Err(e) => {
                tracing::error!("get_group_tree: {}", e);
                Vec::new()
            }
        }
    }

    // ─── Region methods ──────────────────────────────────────────────

    pub async fn list_regions(&self) -> Vec<Region> {
        let conn = match self.rb.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("list_regions acquire: {}", e);
                return Vec::new();
            }
        };
        match region_select_all(conn.rb_ref()).await {
            Ok(rows) => rows.into_iter().map(|r| r.to_region()).collect(),
            Err(e) => {
                tracing::error!("list_regions: {}", e);
                Vec::new()
            }
        }
    }

    pub async fn list_region_children(&self, parent_code: &str) -> Vec<Region> {
        let conn = match self.rb.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("list_region_children acquire: {}", e);
                return Vec::new();
            }
        };
        match region_select_children(conn.rb_ref(), parent_code).await {
            Ok(rows) => rows.into_iter().map(|r| r.to_region()).collect(),
            Err(e) => {
                tracing::error!("list_region_children: {}", e);
                Vec::new()
            }
        }
    }

    pub async fn get_region_tree(&self) -> Vec<RegionNode> {
        let conn = match self.rb.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("get_region_tree acquire: {}", e);
                return Vec::new();
            }
        };
        match region_select_all(conn.rb_ref()).await {
            Ok(rows) => {
                let nodes: Vec<RegionNode> = rows.into_iter()
                    .map(|r| r.to_node())
                    .collect();
                fn build_children(nodes: &[RegionNode], parent_code: &str, visited: &mut std::collections::HashSet<String>) -> Vec<RegionNode> {
                    let mut result = Vec::new();
                    for n in nodes.iter() {
                        if n.parent_code.as_deref() == Some(parent_code) && !visited.contains(&n.code) {
                            visited.insert(n.code.clone());
                            let mut node = n.clone();
                            node.children = build_children(nodes, &n.code, visited);
                            visited.remove(&n.code);
                            result.push(node);
                        }
                    }
                    result
                }
                let roots: Vec<RegionNode> = nodes.iter()
                    .filter(|n| n.parent_code.is_none())
                    .cloned()
                    .map(|mut n| {
                        n.children = build_children(&nodes, &n.code, &mut Default::default());
                        n
                    })
                    .collect();
                roots
            }
            Err(e) => {
                tracing::error!("get_region_tree: {}", e);
                Vec::new()
            }
        }
    }

    // ─── Server methods ───────────────────────────────────────────────

    pub fn servers_cache(&self) -> &Arc<DashMap<i64, ServerRow>> {
        &self.servers_cache
    }

    pub async fn create_server(&self, server: &Server) -> Result<i64> {
        let protocol_ports_json = serde_json::to_value(&server.protocol_ports).unwrap_or(serde_json::Value::Object(Default::default()));
        server_insert(
            self.rb.acquire().await?.rb_ref(),
            Some(&server.name),
            Some(&server.url),
            Some(&server.api_key),
            Some(&server.server_type.to_string()),
            Some(server.weight as i32),
            Some(server.enabled),
            Some(&server.server_tag),
            Some(protocol_ports_json),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;

        // 查询刚插入的记录获取实际 id
        let rows = server_select_by_tag(self.rb.acquire().await?.rb_ref(), &server.server_tag)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let row = rows.into_iter().next()
            .ok_or_else(|| AppError::Internal("Failed to get inserted server".to_string()))?;

        self.servers_cache.insert(row.id, row.clone());
        Ok(row.id)
    }

    pub async fn update_server(&self, server: &Server) -> Result<()> {
        let protocol_ports_json = serde_json::to_value(&server.protocol_ports).unwrap_or(serde_json::Value::Object(Default::default()));
        server_update(
            self.rb.acquire().await?.rb_ref(),
            server.id,
            &server.name,
            &server.url,
            &server.api_key,
            &server.server_type.to_string(),
            server.weight as i32,
            server.enabled,
            &server.server_tag,
            protocol_ports_json,
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;

        self.servers_cache.insert(server.id, ServerRow {
            id: server.id,
            name: server.name.clone(),
            url: server.url.clone(),
            api_key: server.api_key.clone(),
            server_type: server.server_type.to_string(),
            weight: server.weight as i32,
            enabled: server.enabled,
            server_tag: server.server_tag.clone(),
            protocol_ports: serde_json::to_value(&server.protocol_ports).unwrap_or_default(),
            created_at: RbatisDateTime::from_timestamp_millis(server.created_at.timestamp_millis()),
            updated_at: RbatisDateTime::from_timestamp_millis(server.updated_at.timestamp_millis()),
        });
        Ok(())
    }

    pub async fn delete_server(&self, id: i64) -> Result<()> {
        server_delete_by_id(self.rb.acquire().await?.rb_ref(), id)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        self.servers_cache.remove(&id);
        Ok(())
    }

    pub async fn server_exists_by_tag(&self, tag: &str) -> anyhow::Result<bool> {
        match server_select_by_tag(self.rb.acquire().await?.rb_ref(), tag).await {
            Ok(rows) => Ok(!rows.is_empty()),
            Err(e) => {
                tracing::warn!("[DbRepository] server_exists_by_tag failed: {}", e);
                Ok(false)
            }
        }
    }

    // ─── Session methods ──────────────────────────────────────────────

    pub fn sessions_cache(&self) -> &Arc<DashMap<i64, Session>> {
        &self.sessions_cache
    }

    pub async fn create_session(&self, session: &Session) -> Result<()> {
        let session_type_str = session.session_type.to_string();
        let user_id_str = session.user_id.to_string();
        let state_str = session.state.to_string();
        let protocol_str = session.protocol.as_ref().map(|p| p.to_string());
        let expires_rb = session.expires_at.map(|dt| RbatisDateTime::from_timestamp_millis(dt.timestamp_millis()));
        let created_at_rb = RbatisDateTime::from_timestamp_millis(session.created_at.timestamp_millis());
        let last_activity_rb = RbatisDateTime::from_timestamp_millis(session.last_activity.timestamp_millis());
        session_insert(
            self.rb.acquire().await?.rb_ref(),
            Some(&session_type_str),
            session.device_tag.as_deref(),
            session.channel_tag.as_deref(),
            Some(&user_id_str),
            Some(&state_str),
            session.client_ip.as_deref(),
            session.client_type.as_deref(),
            session.media_server_tag.as_deref(),
            protocol_str.as_deref(),
            Some(created_at_rb),
            Some(last_activity_rb),
            expires_rb,
            Some(session.bytes_sent as i64),
            Some(session.bytes_received as i64),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        self.sessions_cache.insert(session.id, session.clone());
        Ok(())
    }

    pub async fn list_sessions_paginated(&self, limit: usize, offset: usize) -> Vec<Session> {
        self.sessions_cache.iter()
            .skip(offset)
            .take(limit)
            .map(|r| r.clone())
            .collect()
    }

    pub async fn count_sessions(&self) -> usize {
        self.sessions_cache.len()
    }

    pub async fn delete_session(&self, id: i64) -> Result<()> {
        session_delete_by_id(self.rb.acquire().await?.rb_ref(), id)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        self.sessions_cache.remove(&id);
        Ok(())
    }

    // ─── Stream methods ──────────────────────────────────────────────

    pub fn streams_cache(&self) -> &Arc<DashMap<String, Stream>> {
        &self.streams_cache
    }

    pub async fn create_stream(&self, stream: &Stream) -> Result<()> {
        let id = stream_insert(
            self.rb.acquire().await?.rb_ref(),
            stream.device_tag.as_deref(),
            stream.channel_tag.as_deref(),
            Some(stream.media_server_tag.clone()),
            &stream.app,
            Some(&stream.token),
            &stream.state.to_string(),
            Some(stream.retry_count as i32),
            Some(stream.max_retries as i32),
            stream.last_error.clone(),
            Some(stream.viewer_count as i32),
            Some(stream.bandwidth_in as i64),
            Some(stream.bandwidth_out as i64),
            RbatisDateTime::from_timestamp_millis(stream.last_keepalive_at.timestamp_millis()),
            RbatisDateTime::from_timestamp_millis(stream.created_at.timestamp_millis()),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        let id = id.ok_or_else(|| AppError::Internal("stream insert returned no id".to_string()))?;
        let mut stream = stream.clone();
        stream.id = id;
        let cache_key = format_cache_key(stream.device_tag.as_deref(), stream.channel_tag.as_deref());
        self.streams_cache.insert(cache_key, stream);
        Ok(())
    }

    pub async fn delete_streams_by_device(&self, device_tag: &str) -> Result<()> {
        let cache_key_prefix = format!("{}/", device_tag);
        self.streams_cache.retain(|k, _| !k.starts_with(&cache_key_prefix));
        Ok(())
    }

    pub fn get_stream_cache_key_by_stream_key(&self, stream_key: &str) -> Option<String> {
        let cache_key = stream_key_to_cache_key(stream_key);
        if self.streams_cache.contains_key(&cache_key) {
            Some(cache_key)
        } else {
            None
        }
    }

    pub async fn update_stream(&self, stream: &Stream) -> Result<()> {
        let conn = self.rb.acquire().await?;
        let update_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream_update(
                conn.rb_ref(),
                stream.device_tag.as_deref(),
                stream.channel_tag.as_deref(),
                Some(stream.media_server_tag.clone()),
                Some(&stream.state.to_string()),
                Some(stream.retry_count as i32),
                Some(stream.max_retries as i32),
                stream.last_error.clone(),
                Some(stream.viewer_count as i32),
                Some(stream.bandwidth_in as i64),
                Some(stream.bandwidth_out as i64),
                Some(RbatisDateTime::from_timestamp_millis(stream.last_keepalive_at.timestamp_millis())),
                stream.id,
            )
        ).await;
        update_result.map_err(|_| AppError::Internal("update_stream timeout".to_string()))?
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let cache_key = format_cache_key(stream.device_tag.as_deref(), stream.channel_tag.as_deref());
        self.streams_cache.insert(cache_key, stream.clone());
        Ok(())
    }

    pub async fn get_stream_by_token(&self, token: &str) -> Result<Option<Stream>> {
        let rows = stream_select_by_token(
            self.rb.acquire().await?.rb_ref(),
            token,
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        match rows.first() {
            Some(row) => Ok(Some(Stream::try_from(row.clone())?)),
            None => Ok(None),
        }
    }

    pub async fn list_streams(&self) -> Vec<Stream> {
        self.streams_cache.iter().map(|r| r.clone()).collect()
    }

    pub async fn list_streams_paginated(&self, limit: usize, offset: usize) -> Vec<Stream> {
        self.streams_cache.iter()
            .skip(offset)
            .take(limit)
            .map(|r| r.clone())
            .collect()
    }

    pub async fn count_streams(&self) -> usize {
        self.streams_cache.len()
    }

    // ─── Recording methods ─────────────────────────────────────────────

    pub async fn create_recording(&self, recording: &Recording) -> Result<()> {
        recording_insert(
            self.rb.acquire().await?.rb_ref(),
            recording.id,
            recording.device_tag.as_deref(),
            recording.channel_tag.as_deref(),
            &recording.media_server_name,
            &recording.state.to_string(),
            &recording.format.to_string(),
            recording.output_path.clone(),
            recording.file_size as i64,
            recording.duration_secs as i64,
            RbatisDateTime::from_timestamp_millis(recording.created_at.timestamp_millis()),
            recording.started_at.map(|dt| RbatisDateTime::from_timestamp_millis(dt.timestamp_millis())),
            recording.stopped_at.map(|dt| RbatisDateTime::from_timestamp_millis(dt.timestamp_millis())),
            recording.error_message.clone(),
            serde_json::to_string(&recording.labels).ok(),
            recording.filename.clone(),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn update_recording(&self, recording: &Recording) -> Result<()> {
        recording_update(
            self.rb.acquire().await?.rb_ref(),
            recording.device_tag.as_deref(),
            recording.channel_tag.as_deref(),
            Some(recording.media_server_name.clone()),
            Some(recording.state.to_string()),
            Some(recording.format.to_string()),
            recording.output_path.clone(),
            Some(recording.file_size as i64),
            Some(recording.duration_secs as i64),
            recording.started_at.map(|dt| RbatisDateTime::from_timestamp_millis(dt.timestamp_millis())),
            recording.stopped_at.map(|dt| RbatisDateTime::from_timestamp_millis(dt.timestamp_millis())),
            recording.error_message.clone(),
            serde_json::to_string(&recording.labels).ok(),
            recording.filename.clone(),
            recording.id,
        ).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn list_recordings_paginated(&self, limit: usize, offset: usize) -> Result<Vec<Recording>> {
        let rows = recording_select_paginated(self.rb.acquire().await?.rb_ref(), limit as i64, offset as i64)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        let recordings: Vec<Recording> = rows.into_iter().filter_map(|r| Recording::try_from(r).ok()).collect();
        Ok(recordings)
    }

    pub async fn count_recordings(&self) -> Result<usize> {
        let count = recording_count_all(self.rb.acquire().await?.rb_ref())
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(count as usize)
    }

    pub async fn find_active_recording_by_stream_key(&self, stream_key: &str) -> Result<Option<Recording>> {
        let rows = recording_select_all(self.rb.acquire().await?.rb_ref())
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        let recording = rows.into_iter()
            .filter_map(|r| Recording::try_from(r).ok())
            .find(|r| {
                r.stream_key() == stream_key
                    && (r.state == crate::domain::recording::RecordingState::Recording
                        || r.state == crate::domain::recording::RecordingState::Starting)
            });
        Ok(recording)
    }

    pub async fn get_recording(&self, id: i64) -> Result<Option<Recording>> {
        let rows = recording_select_by_id(self.rb.acquire().await?.rb_ref(), id)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(rows.into_iter().filter_map(|r| Recording::try_from(r).ok()).next())
    }

    pub async fn delete_recording(&self, id: i64) -> Result<()> {
        recording_delete_by_id(self.rb.acquire().await?.rb_ref(), id)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn list_recordings_by_device(&self, device_tag: &str) -> Result<Vec<Recording>> {
        self.list_recordings_by_device_tag(device_tag).await
    }

    pub async fn list_recordings_by_device_tag(&self, device_tag: &str) -> Result<Vec<Recording>> {
        let rows = recording_select_by_device_tag(self.rb.acquire().await?.rb_ref(), device_tag)
            .await.map_err(|e| AppError::Internal(e.to_string()))?;
        let recordings: Vec<Recording> = rows.into_iter()
            .filter_map(|r| Recording::try_from(r).ok())
            .collect();
        Ok(recordings)
    }

    // ─── PTZ methods ─────────────────────────────────────────────────

    pub async fn log_ptz_control(
        &self,
        user_id: Option<String>,
        device_id: i64,
        command: &str,
        speed: f64,
        result: bool,
        call_id: Option<String>,
        error_message: Option<String>,
    ) -> Result<()> {
        let _ = (user_id, device_id, command, speed, result, call_id, error_message);
        Ok(())
    }

    pub async fn log_ptz_result(
        &self,
        device_id: i64,
        call_id: Option<&str>,
        sip_code: Option<u16>,
        status: &str,
        message: Option<String>,
    ) -> Result<()> {
        let _ = (device_id, call_id, sip_code, status, message);
        Ok(())
    }

    pub async fn list_ptz_presets(&self, _device_id: i64) -> Result<Vec<PtzPreset>> {
        Ok(Vec::new())
    }

    pub async fn create_ptz_preset(&self, device_id: i64, name: &str) -> Result<PtzPreset> {
        let token = uuid::Uuid::new_v4().to_string();
        let preset = PtzPreset::new(device_id, name.to_string(), token);
        Ok(preset)
    }

    pub async fn create_ptz_preset_with_token(
        &self,
        device_id: i64,
        name: &str,
        token: &str,
    ) -> Result<PtzPreset> {
        let preset = PtzPreset::new(device_id, name.to_string(), token.to_string());
        Ok(preset)
    }

    pub async fn update_ptz_preset(&self, device_id: i64, token: &str, name: &str) -> Result<()> {
        let device_uuid = {
            let conn = self.rb.acquire().await?;
            let rows: Vec<crate::sql_mappers::DeviceRow> = crate::sql_mappers::device_select_by_id(conn.rb_ref(), device_id).await?;
            rows.into_iter().next()
                .map(|r| r.id.to_string())
                .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_id)))?
        };
        crate::sql_mappers::ptz_preset_update(self.rb.acquire().await?.rb_ref(), device_uuid, name.to_string(), token.to_string()).await?;
        Ok(())
    }

    pub async fn delete_ptz_preset(&self, _device_id: i64, _token: &str) -> Result<()> {
        Ok(())
    }

    // ─── Layout methods ───────────────────────────────────────────────

    pub async fn get_layout(&self, id: i64) -> Option<PlayerLayout> {
        layout_select_by_id(&self.rb, id).await.ok()
            .and_then(|rows| rows.into_iter().next().map(PlayerLayout::from))
    }

    pub async fn list_layouts(&self) -> Vec<PlayerLayout> {
        layout_select_all(&self.rb).await.unwrap_or_default()
            .into_iter().map(PlayerLayout::from).collect()
    }

    pub async fn get_default_layout(&self) -> Option<PlayerLayout> {
        layout_select_default(&self.rb).await.ok()
            .and_then(|rows| rows.into_iter().next().map(PlayerLayout::from))
    }

    pub async fn set_default_layout(&self, id: i64) -> Result<()> {
        layout_set_default(&self.rb).await.map_err(|e| AppError::Internal(e.to_string()))?;
        layout_update(&self.rb, id, None::<&str>, None, None, None, Some(true)).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn create_layout(&self, name: &str, rows: i32, cols: i32, layout_json: serde_json::Value, is_default: bool) -> Result<i64> {
        if is_default {
            let _ = layout_set_default(&self.rb).await;
        }
        let rows = layout_insert(&self.rb, name, rows, cols, layout_json, is_default).await?;
        rows.into_iter().next().map(|r| r.id).ok_or_else(|| AppError::Internal("Failed to insert layout".to_string()))
    }

    pub async fn update_layout(
        &self,
        id: i64,
        name: Option<String>,
        rows: Option<i32>,
        cols: Option<i32>,
        layout_json: Option<serde_json::Value>,
        is_default: Option<bool>,
    ) -> Result<()> {
        if is_default == Some(true) {
            let _ = layout_set_default(&self.rb).await;
        }
        let name_ref = name.as_deref();
        layout_update(&self.rb, id, name_ref, rows, cols, layout_json, is_default).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_layout(&self, id: i64) -> Result<()> {
        layout_delete(&self.rb, id).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }
}

// ─── Helper functions ───────────────────────────────────────────────

fn protocol_to_str(p: &Protocol) -> &'static str {
    match p {
        Protocol::Gb28181 => "GB28181",
        Protocol::Onvif => "ONVIF",
        Protocol::Rtsp => "RTSP",
        Protocol::Rtmp => "RTMP",
        Protocol::Hls => "HLS",
        Protocol::WebRTC => "WebRTC",
    }
}

fn status_to_str(s: &DeviceStatus) -> &'static str {
    match s {
        DeviceStatus::Offline => "offline",
        DeviceStatus::Online => "online",
        DeviceStatus::Error => "error",
        DeviceStatus::Maintaining => "maintaining",
    }
}

fn build_group_tree(rows: Vec<GroupRow>) -> Vec<DeviceGroupNode> {
    let nodes: Vec<DeviceGroupNode> = rows.iter()
        .filter_map(|r| DeviceGroupNode::try_from(r.clone()).ok())
        .collect();

    fn build_children(nodes: &[DeviceGroupNode], parent_id: Option<i64>, visited: &mut std::collections::HashSet<i64>) -> Vec<DeviceGroupNode> {
        let mut result = Vec::new();
        for n in nodes.iter() {
            if n.parent_id == parent_id && !visited.contains(&n.id) {
                visited.insert(n.id);
                let mut node = n.clone();
                node.children = build_children(nodes, Some(n.id), visited);
                visited.remove(&n.id);
                result.push(node);
            }
        }
        result
    }

    build_children(&nodes, None, &mut Default::default())
}

impl TryFrom<SessionRow> for Session {
    type Error = AppError;

    fn try_from(row: SessionRow) -> Result<Self> {
        use crate::domain::{SessionState, SessionType};

        let session_type = match row.session_type.as_str() {
            "play" => SessionType::Play,
            "record" => SessionType::Record,
            "playback" => SessionType::Playback,
            "ptz" => SessionType::Ptz,
            other => return Err(AppError::Internal(format!("unknown session_type: {other}"))),
        };

        let state = match row.state.as_str() {
            "initializing" => SessionState::Initializing,
            "active" => SessionState::Active,
            "idle" => SessionState::Idle,
            "terminating" => SessionState::Terminating,
            "terminated" => SessionState::Terminated,
            other => return Err(AppError::Internal(format!("unknown session state: {other}"))),
        };

        let user_id = row.user_id.parse::<i64>()
            .map_err(|e| AppError::Internal(format!("invalid user_id: {e}")))?;

        let protocol = match row.protocol.as_ref() {
            Some(p) => Some(match p.as_str() {
                "GB28181" => Protocol::Gb28181,
                "ONVIF" => Protocol::Onvif,
                "RTSP" => Protocol::Rtsp,
                "RTMP" => Protocol::Rtmp,
                "HLS" => Protocol::Hls,
                "WebRTC" => Protocol::WebRTC,
                other => return Err(AppError::Internal(format!("unknown protocol: {other}"))),
            }),
            None => None,
        };

        let created_at = chrono::DateTime::from_timestamp(row.created_at.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);

        let last_activity = chrono::DateTime::from_timestamp(row.last_activity.unix_timestamp(), 0)
            .unwrap_or_else(chrono::Utc::now);

        let expires_at = row.expires_at.and_then(|s| chrono::DateTime::from_timestamp(s.unix_timestamp(), 0));

        Ok(Session {
            id: row.id,
            session_type,
            device_tag: row.device_tag,
            channel_tag: row.channel_tag,
            user_id,
            state,
            client_ip: row.client_ip,
            client_type: row.client_type,
            media_server_tag: row.media_server_tag,
            protocol,
            created_at,
            last_activity,
            expires_at,
            bytes_sent: row.bytes_sent as u64,
            bytes_received: row.bytes_received as u64,
        })
    }
}

impl DbRepository {
    pub async fn save_alarm(&self, alarm: &CreateAlarmRequest) -> Result<Alarm> {
        let alarm_time = RbatisDateTime::from_timestamp_millis(alarm.alarm_time.timestamp_millis());
        let rows = alarm_insert(
            self.rb.acquire().await?.rb_ref(),
            alarm.device_id,
            &alarm.device_tag,
            &alarm.alarm_type,
            alarm_time,
            alarm.alarm_method.unwrap_or(1),
            alarm.alarm_priority.unwrap_or(0),
            alarm.description.as_deref(),
        ).await.map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        rows.into_iter().next().map(|r| r.into()).ok_or_else(|| AppError::Internal("Failed to insert alarm".to_string()))
    }

    pub async fn list_alarms(&self, device_id: Option<i64>, limit: usize, offset: usize) -> Result<Vec<Alarm>> {
        let limit = limit as i64;
        let offset = offset as i64;
        let rows = if let Some(did) = device_id {
            alarm_select_by_device(self.rb.acquire().await?.rb_ref(), did, limit, offset)
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        } else {
            alarm_select_all(self.rb.acquire().await?.rb_ref(), limit, offset)
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        };
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count_alarms(&self, device_id: Option<i64>) -> Result<i64> {
        let count = if let Some(did) = device_id {
            alarm_count_by_device(self.rb.acquire().await?.rb_ref(), did)
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        } else {
            alarm_count_all(self.rb.acquire().await?.rb_ref())
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        };
        Ok(count)
    }

    pub async fn mark_alarm_processed(&self, alarm_id: i64, processed: bool) -> Result<()> {
        alarm_mark_processed(self.rb.acquire().await?.rb_ref(), processed, alarm_id)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
