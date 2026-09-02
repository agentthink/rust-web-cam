use std::sync::Arc;
use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::state::FullState;
use crate::api::ApiResponse;
use crate::error::AppError;
use crate::protocol::onvif::OnvifProbeService;

#[derive(Debug, Deserialize)]
pub struct OnvifProbeRequest {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct OnvifCapabilitiesRequest {
    pub x_addr: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct OnvifStreamUriRequest {
    pub x_addr: String,
    pub media_x_addr: Option<String>,
    pub username: String,
    pub password: String,
    pub profiles: Vec<String>,
}

pub async fn onvif_discover_handler(
    State(_state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let devices = OnvifProbeService::discover_multicast().await
        .map_err(|e| AppError::Internal(format!("ONVIF discovery failed: {}", e)))?;

    let result: Vec<serde_json::Value> = devices.into_iter().map(|d| serde_json::json!({
        "host": d.host,
        "port": d.port,
        "urn": d.urn,
        "x_addr": d.x_addr,
        "name": d.name,
        "manufacturer": d.manufacturer,
        "model": d.model,
    })).collect();

    Ok(Json(ApiResponse::success(result)))
}

pub async fn onvif_probe_handler(
    State(_state): State<Arc<FullState>>,
    Json(req): Json<OnvifProbeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let device = OnvifProbeService::probe_unicast(&req.host, req.port).await
        .map_err(|e| AppError::Internal(format!("ONVIF probe failed: {}", e)))?
        .ok_or_else(|| AppError::NotFound(format!("No ONVIF device found at {}:{}", req.host, req.port)))?;

    let result = serde_json::json!({
        "host": device.host,
        "port": device.port,
        "urn": device.urn,
        "x_addr": device.x_addr,
        "name": device.name,
        "manufacturer": device.manufacturer,
        "model": device.model,
    });

    Ok(Json(ApiResponse::success(result)))
}

pub async fn onvif_capabilities_handler(
    State(_state): State<Arc<FullState>>,
    Json(req): Json<OnvifCapabilitiesRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let caps = OnvifProbeService::get_capabilities(&req.x_addr, &req.username, &req.password).await
        .map_err(|e| AppError::BadRequest(format!("Failed to get capabilities: {}", e)))?;

    let result = serde_json::json!({
        "device_info": {
            "manufacturer": caps.device_info.manufacturer,
            "model": caps.device_info.model,
            "firmware_version": caps.device_info.firmware_version,
            "serial_number": caps.device_info.serial_number,
            "hardware_id": caps.device_info.hardware_id,
        },
        "capabilities": {
            "media": caps.capabilities.media,
            "ptz": caps.capabilities.ptz,
            "events": caps.capabilities.events,
            "imaging": caps.capabilities.imaging,
        },
        "profiles": caps.profiles,
    });

    Ok(Json(ApiResponse::success(result)))
}

pub async fn onvif_stream_uris_handler(
    State(_state): State<Arc<FullState>>,
    Json(req): Json<OnvifStreamUriRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = OnvifProbeService::get_stream_uris(
        req.media_x_addr.as_deref(),
        &req.x_addr,
        &req.username,
        &req.password,
        &req.profiles,
    ).await
        .map_err(|e| AppError::BadRequest(format!("Failed to get stream URIs: {}", e)))?;

    let streams: Vec<serde_json::Value> = result.streams.into_iter().map(|s| serde_json::json!({
        "token": s.token,
        "name": s.name,
        "rtsp_url": s.rtsp_url,
    })).collect();

    Ok(Json(ApiResponse::success(serde_json::json!({ "streams": streams }))))
}

#[derive(Debug, Deserialize)]
pub struct OnvifCreateDevicesRequest {
    pub urn: Option<String>,
    pub x_addr: String,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub username: String,
    pub password: String,
    pub capabilities: OnvifCapabilityUrls,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub streams: Vec<OnvifStreamToCreate>,
}

#[derive(Debug, Deserialize)]
pub struct OnvifCapabilityUrls {
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default)]
    pub ptz: Option<String>,
    #[serde(default)]
    pub events: Option<String>,
    #[serde(default)]
    pub imaging: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OnvifStreamToCreate {
    pub token: String,
    pub name: String,
    pub rtsp_url: String,
}

pub async fn onvif_create_devices_handler(
    State(state): State<Arc<FullState>>,
    Json(req): Json<OnvifCreateDevicesRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if req.streams.is_empty() {
        return Err(AppError::BadRequest("At least one stream is required".to_string()));
    }

    let result = state.app.registry.device_service
        .create_onvif_devices(
            req.urn.as_deref(),
            &req.x_addr,
            req.name.as_deref(),
            req.manufacturer.as_deref(),
            req.model.as_deref(),
            &req.username,
            &req.password,
            &req.capabilities,
            &req.types,
            &req.scopes,
            &req.streams,
        )
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to create ONVIF devices: {}", e)))?;

    Ok(Json(ApiResponse::success(result)))
}

#[derive(Debug, Deserialize)]
pub struct OnvifCheckOnlineRequest {
    pub x_addr: String,
    pub username: String,
    pub password: String,
}

pub async fn onvif_check_online_handler(
    State(_state): State<Arc<FullState>>,
    Json(req): Json<OnvifCheckOnlineRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let online = OnvifProbeService::check_online(&req.x_addr, &req.username, &req.password).await;
    Ok(Json(ApiResponse::success(serde_json::json!({ "online": online }))))
}
