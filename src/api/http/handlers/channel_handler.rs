use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::api::response::ApiResponse;
use crate::api::state::FullState;
use crate::application::ChannelService;
use crate::domain::{Channel, DeviceStatus, DeviceType, Protocol};
use crate::error::AppError;
use crate::protocol::adapter_manager;

#[derive(Debug, Deserialize)]
pub struct ChannelPaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub device_tag: Option<String>,
    pub status: Option<String>,
}

impl ChannelPaginationQuery {
    pub fn limit(&self) -> usize { self.limit.unwrap_or(50).min(500) }
    pub fn offset(&self) -> usize { self.offset.unwrap_or(0) }
}

#[derive(Debug, Serialize)]
pub struct PaginatedChannels {
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub items: Vec<Channel>,
}

pub async fn list_channels_handler(
    State(state): State<Arc<FullState>>,
    Query(q): Query<ChannelPaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedChannels>>, AppError> {
    let all_channels = state.app.registry.channel_service.list_channels_cached();
    
    let filtered: Vec<Channel> = if let Some(ref dt) = q.device_tag {
        all_channels.iter()
            .filter(|c| &c.device_tag == dt)
            .cloned()
            .collect()
    } else {
        all_channels
    };

    let total = filtered.len();
    let items: Vec<Channel> = filtered.into_iter()
        .skip(q.offset())
        .take(q.limit())
        .collect();

    Ok(Json(ApiResponse::success(PaginatedChannels {
        total,
        limit: q.limit(),
        offset: q.offset(),
        items,
    })))
}

pub async fn get_channel_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Channel>>, AppError> {
    let channel = state.app.registry.channel_service
        .get_channel(&device_tag, &channel_tag)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Channel {}/{} not found", device_tag, channel_tag)))?;
    Ok(Json(ApiResponse::success(channel)))
}

pub async fn get_channels_by_device_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
) -> Result<Json<ApiResponse<Vec<Channel>>>, AppError> {
    let channels = state.app.registry.channel_service.get_channels_by_device(&device_tag);
    Ok(Json(ApiResponse::success(channels)))
}

#[derive(Debug, serde::Deserialize)]
pub struct ChannelPlayQuery {
    pub stream_key: Option<String>,
}

pub async fn get_channel_play_links_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
    Query(query): Query<ChannelPlayQuery>,
) -> Result<Json<ApiResponse<crate::domain::device::PlayLinks>>, AppError> {
    let channel = state.app.registry.channel_service
        .get_channel(&device_tag, &channel_tag)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Channel {}/{} not found", device_tag, channel_tag)))?;

    let cache = state.app.registry.infra.db.streams_cache();
    let stream_key = query.stream_key.unwrap_or_else(|| channel.channel_tag.clone());
    
    let stream_id = crate::domain::stream::make_stream_key(&device_tag, stream_key.as_str());

    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    let token = state.app.registry.play_service.generate_token(&device.id.to_string()).await?;
    let play_links = state.app.registry.play_service.build_play_links(&device, &token, &stream_id).await;

    Ok(Json(ApiResponse::success(play_links)))
}

/// POST /api/v1/channels/{device_tag}/{channel_tag}/start
/// 启动 GB28181 通道的流（发送 INVITE）
pub async fn start_channel_stream_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
) -> Result<Json<ApiResponse<StartChannelResult>>, AppError> {
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    
    if device.protocol != Protocol::Gb28181 {
        return Err(AppError::BadRequest("Only GB28181 devices support this operation".to_string()));
    }
    
    let adapter = adapter_manager::get_adapter(&device_tag)
        .ok_or_else(|| AppError::BadRequest(format!("GB28181 adapter not found for device {}", device_tag)))?;
    
    let mut guard = adapter.lock().await;
    guard.start(&channel_tag).await
        .map_err(|e| AppError::Internal(format!("Failed to start stream: {}", e)))?;
    
    tracing::info!("[Channel] Started GB28181 stream: device={}, channel={}", device_tag, channel_tag);
    
    Ok(Json(ApiResponse::success(StartChannelResult {
        device_tag: device_tag.clone(),
        channel_tag: channel_tag.clone(),
        message: "INVITE sent, waiting for device response".to_string(),
    })))
}

#[derive(Debug, Serialize)]
pub struct StartChannelResult {
    pub device_tag: String,
    pub channel_tag: String,
    pub message: String,
}

pub async fn get_channel_status_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ChannelStatus>>, AppError> {
    let channel = state.app.registry.channel_service
        .get_channel(&device_tag, &channel_tag)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Channel {}/{} not found", device_tag, channel_tag)))?;

    let cache = state.app.registry.infra.db.streams_cache();
    let has_stream = cache.iter()
        .any(|s| s.channel_tag.as_ref() == Some(&channel.channel_tag));

    let status = ChannelStatus {
        channel_tag: channel.channel_tag,
        device_tag: channel.device_tag,
        name: channel.name,
        status: channel.status,
        has_stream,
        stream_count: if has_stream { 1 } else { 0 },
    };

    Ok(Json(ApiResponse::success(status)))
}

#[derive(Debug, serde::Serialize)]
pub struct ChannelStatus {
    pub channel_tag: String,
    pub device_tag: String,
    pub name: String,
    pub status: DeviceStatus,
    pub has_stream: bool,
    pub stream_count: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct DeviceChannelsQuery {
    pub status: Option<String>,
}

pub async fn get_device_channels_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
    Query(_query): Query<DeviceChannelsQuery>,
) -> Result<Json<ApiResponse<Vec<Channel>>>, AppError> {
    let channels = state.app.registry.channel_service.get_channels_by_device(&device_tag);
    Ok(Json(ApiResponse::success(channels)))
}
