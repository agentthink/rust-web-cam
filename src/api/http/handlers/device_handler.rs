use std::sync::Arc;
use axum::{
    extract::{Path, State, Query, Extension},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::api::state::FullState;
use crate::api::response::ApiResponse;
use crate::error::AppError;
use crate::domain::{Device, CreateDeviceRequest, UpdateDeviceRequest, Protocol, DeviceWithChildren};
use crate::auth::CurrentUser;
use crate::protocol::onvif::OnvifDeviceClient;

fn inject_rtsp_auth(url: &str, username: &str, password: &str) -> String {
    if username.is_empty() && password.is_empty() {
        return url.to_string();
    }
    if url.starts_with("rtsp://") {
        let without_scheme = &url[7..];
        if without_scheme.contains('@') {
            return url.to_string();
        }
        if let Some(first_slash) = without_scheme.find('/') {
            let host_part = &without_scheme[..first_slash];
            let path_part = &without_scheme[first_slash..];
            return format!("rtsp://{}:{}@{}{}", username, password, host_part, path_part);
        } else {
            return format!("rtsp://{}:{}@{}", username, password, without_scheme);
        }
    }
    url.to_string()
}

/// 分页查询参数
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search: Option<String>,
}

impl PaginationQuery {
    pub fn limit(&self) -> usize { self.limit.unwrap_or(50).min(500) }
    pub fn offset(&self) -> usize { self.offset.unwrap_or(0) }
}

/// 播放请求参数
#[derive(Debug, Deserialize)]
pub struct PlayRequest {
    pub protocol: Option<Protocol>,
}

// ═══════════════════════════════════════════════════════════════
// 设备 CRUD Handlers
// ═══════════════════════════════════════════════════════════════

/// GET /api/v1/devices
pub async fn list_devices_handler(
    State(state): State<Arc<FullState>>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<Device>>>, AppError> {
    let registry = &state.app.registry;
    let devices = registry.device_service.list_devices_paginated(q.limit(), q.offset(), q.search.as_deref()).await;
    let total = registry.device_service.count_devices_filtered(q.search.as_deref()).await;

    Ok(Json(ApiResponse::success(PaginatedResponse {
        items: devices,
        total,
        limit: q.limit(),
        offset: q.offset(),
    })))
}

/// GET /api/v1/devices/online
pub async fn list_online_devices_handler(
    State(state): State<Arc<FullState>>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<Device>>>, AppError> {
    let registry = &state.app.registry;
    let devices = registry.infra.db.list_online_devices_paginated(q.limit(), q.offset()).await;
    let total = registry.infra.db.count_online_devices().await;

    Ok(Json(ApiResponse::success(PaginatedResponse {
        items: devices,
        total,
        limit: q.limit(),
        offset: q.offset(),
    })))
}

/// POST /api/v1/devices
pub async fn create_device_handler(
    State(state): State<Arc<FullState>>,
    Json(req): Json<CreateDeviceRequest>,
) -> Result<Json<ApiResponse<Device>>, AppError> {
    let device = state.app.registry.device_service.create_device(req).await?;
    Ok(Json(ApiResponse::success(device)))
}

/// GET /api/v1/devices/:device_tag
pub async fn get_device_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
) -> Result<Json<ApiResponse<Device>>, AppError> {
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    Ok(Json(ApiResponse::success(device)))
}

/// PUT /api/v1/devices/:id
pub async fn update_device_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateDeviceRequest>,
) -> Result<Json<ApiResponse<Device>>, AppError> {
    let updated = state.app.registry.device_service.update_device(id, req).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// DELETE /api/v1/devices/:device_tag
pub async fn delete_device_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let registry = &state.app.registry;

    // 检查设备存在
    let device = registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;

    // 停止关联的流
    let streams = registry.stream_service.list_all_streams().await
        .into_iter()
        .filter(|s| s.device_tag.as_ref() == Some(&device_tag) && s.state == crate::domain::StreamState::Active)
        .collect::<Vec<_>>();

    for stream in streams {
        let stream_key = crate::domain::stream::make_stream_key(stream.device_tag.as_deref().unwrap_or(""), stream.channel_tag.as_deref().unwrap_or(""));
        let _ = registry.stream_service.stop_stream(&stream.app, &stream_key).await;
    }

    // 停止关联的录制
    let recordings = registry.recording_service.list_device_recordings_by_device_tag(&device_tag).await?;
    for recording in recordings {
        if recording.state == crate::domain::RecordingState::Recording {
            let _ = registry.recording_service.stop_recording(recording.id).await;
        }
        let _ = registry.recording_service.delete_recording(recording.id).await;
    }

    registry.device_service.delete_device(&device_tag).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/devices/:device_tag/play
pub async fn play_device_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
    Query(_params): Query<PlayRequest>,
) -> Result<Json<ApiResponse<crate::adapter::media_server::StreamInfo>>, AppError> {
    let registry = &state.app.registry;

    let device = registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;

    let (_, pull_url) = device.select_source()
        .ok_or_else(|| AppError::BadRequest("Device has no stream source configured".to_string()))?;

    let session = registry.session_service.create_session(
        crate::domain::SessionType::Play, device.id,
    ).await?;

    let stream_info = registry.session_service.activate_session(session.id, &pull_url).await?;

    Ok(Json(ApiResponse::success(stream_info)))
}

#[derive(Debug, Deserialize)]
pub struct PlaybackRequest {
    pub start_time: String,
    pub end_time: String,
}

/// POST /api/v1/devices/:id/playback
pub async fn playback_device_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
    Json(req): Json<PlaybackRequest>,
) -> Result<Json<ApiResponse<StartDeviceResult>>, AppError> {
    let registry = &state.app.registry;
    let device = registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;

    let start_dt = chrono::DateTime::parse_from_rfc3339(&req.start_time)
        .map_err(|_| AppError::BadRequest("Invalid start_time format".to_string()))?
        .with_timezone(&chrono::Utc);
    let end_dt = chrono::DateTime::parse_from_rfc3339(&req.end_time)
        .map_err(|_| AppError::BadRequest("Invalid end_time format".to_string()))?
        .with_timezone(&chrono::Utc);

    match device.protocol {
        Protocol::Gb28181 => {
            let device_tag = device.device_tag.clone()
                .ok_or_else(|| AppError::BadRequest(format!("Device {} has no device_tag", device_tag)))?;
            let adapter_key = device.parent_device_tag.clone().unwrap_or(device_tag.clone());
            let adapter_arc = crate::protocol::adapter_manager::get_adapter(&adapter_key)
                .ok_or_else(|| AppError::NotFound(format!("Device {} is offline", device_tag)))?;

            let mut inner = adapter_arc.lock().await;
            (&mut *inner).start_playback(&device_tag, start_dt, end_dt).await
                .map_err(|e| AppError::Internal(format!("Playback INVITE failed: {}", e)))?;

            Ok(Json(ApiResponse::success(StartDeviceResult {
                success: true,
                message: "Playback INVITE sent".to_string(),
            })))
        }
        _ => Err(AppError::BadRequest("Playback only supported for GB28181 devices".to_string()))
    }
}

/// GET /api/v1/devices/:device_tag/play-links
pub async fn get_play_links_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
) -> Result<Json<ApiResponse<crate::domain::device::PlayLinks>>, AppError> {
    let registry = &state.app.registry;

    let device = registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;

    device.select_source()
        .ok_or_else(|| AppError::BadRequest("Device has no stream source configured".to_string()))?;

    let stream_id = if device.parent_device_tag.is_some() {
        crate::domain::stream::make_stream_key(&device_tag, &device_tag)
    } else {
        crate::domain::stream::make_stream_key(&device_tag, "main")
    };

    let token = registry.play_service.generate_token(&device.id.to_string()).await?;
    let play_links = registry.play_service.build_play_links(&device, &token, &stream_id).await;

    Ok(Json(ApiResponse::success(play_links)))
}

/// GET /api/v1/devices/:device_tag/config?type=BasicParam
#[derive(Debug, Deserialize)]
pub struct DeviceConfigQuery {
    pub r#type: Option<String>,
}

pub async fn get_device_config_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
    Query(query): Query<DeviceConfigQuery>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let registry = &state.app.registry;

    let device = registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;

    let config_type = query.r#type.as_deref().unwrap_or("BasicParam");
    tracing::info!("[DeviceConfig] Query device {} config type={}", device_tag, config_type);

    let config_xml = match device.protocol {
        Protocol::Gb28181 => {
            let device_tag = device.device_tag.as_ref().ok_or_else(|| AppError::BadRequest("Device has no device tag".to_string()))?;
            if let Some(adapter) = crate::protocol::adapter_manager::get_adapter(device_tag) {
                let guard = adapter.lock().await;
                let resp = guard.send_device_config_query(config_type).await
                    .map_err(|e| AppError::BadRequest(format!("Config query failed: {}", e)))?;
                resp
            } else {
                format!(r#"<Error>Device {} not connected</Error>"#, device_tag)
            }
        }
        _ => format!(r#"<Error>Protocol {} does not support config query</Error>"#, device.protocol),
    };

    Ok(Json(ApiResponse::success(config_xml)))
}

/// POST /api/v1/devices/:device_tag/stop
pub async fn stop_device_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let registry = &state.app.registry;

    let sessions: Vec<_> = registry.infra.db.sessions_cache().iter()
        .filter(|s| {
            if s.state != crate::domain::SessionState::Active {
                return false;
            }
            if s.device_tag.as_ref() == Some(&device_tag) {
                return true;
            }
            false
        })
        .map(|s| s.clone())
        .collect();

    for session in sessions {
        let _ = registry.session_service.deactivate_session(session.id).await;
    }

    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/devices/:device_tag/start
pub async fn start_device_handler(
    State(state): State<Arc<FullState>>,
    Path(device_tag): Path<String>,
) -> Result<Json<ApiResponse<StartDeviceResult>>, AppError> {
    let registry = &state.app.registry;

    let device = registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;

    match device.protocol {
        Protocol::Gb28181 => {
            let device_tag = device.device_tag.clone()
                .ok_or_else(|| AppError::BadRequest(format!("Device {} has no device_tag", device_tag)))?;
            let adapter_key = device.parent_device_tag.clone().unwrap_or(device_tag.clone());
            let adapter_arc = crate::protocol::adapter_manager::get_adapter(&adapter_key)
                .ok_or_else(|| AppError::NotFound(format!("Device {} is offline", device_tag)))?;

            let mut inner = adapter_arc.lock().await;
            (&mut *inner).start(&device_tag).await
                .map_err(|e| AppError::Internal(format!("INVITE failed: {}", e)))?;

            Ok(Json(ApiResponse::success(StartDeviceResult {
                success: true,
                message: "INVITE sent".to_string(),
            })))
        }
        Protocol::Onvif => {
            let x_addr = device.extended.as_ref()
                .and_then(|e| e.get("x_addr"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("Device has no ONVIF x_addr".to_string()))?;

            let mut client = OnvifDeviceClient::new(x_addr);
            if let (Some(u), Some(p)) = (&device.device_username, &device.device_password) {
                client = client.with_credentials(u, p);
            }

            let streams = client.get_all_stream_uris().await
                .map_err(|e| AppError::Internal(format!("Failed to get ONVIF stream URIs: {}", e)))?;

            if streams.is_empty() {
                return Err(AppError::NotFound("No stream URIs found on ONVIF device".to_string()));
            }

            let device_tag = device.device_tag.clone()
                .ok_or_else(|| AppError::BadRequest(format!("Device has no device_tag")))?;
            let mut started = Vec::new();
            for (profile, uri) in streams {
                let uname = device.device_username.as_deref().unwrap_or("");
                let pwd = device.device_password.as_deref().unwrap_or("");
                let full_url = inject_rtsp_auth(&uri.uri, uname, pwd);
                let stream_info = registry.stream_service.start_pull_stream(&device_tag, &device_tag, &full_url).await
                    .map_err(|e| AppError::Internal(format!("Failed to start stream for profile {}: {}", profile.token, e)))?;
                tracing::info!("[ONVIF] Started stream: profile={} play_url={}", profile.token, stream_info.play_url);
                started.push(stream_info);
            }

            let message = if started.len() == 1 {
                format!("Stream started: {}", started[0].play_url)
            } else {
                format!("{} streams started", started.len())
            };

            Ok(Json(ApiResponse::success(StartDeviceResult {
                success: true,
                message,
            })))
        }
        Protocol::Rtsp => {
            let device_tag = device.device_tag.clone()
                .ok_or_else(|| AppError::BadRequest(format!("Device has no device_tag")))?;
            let raw_url = match device.select_source() {
                Some((_, url)) => url.clone(),
                None => {
                    return Err(AppError::BadRequest("设备未配置推流或拉流地址".to_string()));
                }
            };

            let uname = device.device_username.as_deref().unwrap_or("");
            let pwd = device.device_password.as_deref().unwrap_or("");
            let full_url = inject_rtsp_auth(&raw_url, uname, pwd);

            let stream_info = registry.stream_service.start_pull_stream(&device_tag, &device_tag, &full_url).await
                .map_err(|e| AppError::Internal(format!("Failed to start stream: {}", e)))?;
            Ok(Json(ApiResponse::success(StartDeviceResult {
                success: true,
                message: format!("Stream started: {}", stream_info.play_url),
            })))
        }
        Protocol::Rtmp => {
            return Err(AppError::BadRequest("RTMP 设备需要主动推流到媒体服务器，请确认设备已配置推流".to_string()));
        }
        Protocol::Hls | Protocol::WebRTC => {
            let device_tag = device.device_tag.clone()
                .ok_or_else(|| AppError::BadRequest(format!("Device has no device_tag")))?;
            let url = device.select_source().map(|(_, url)| url)
                .ok_or_else(|| AppError::BadRequest(format!("{} device has no stream source", device.protocol)))?;
            let stream_info = registry.stream_service.start_pull_stream(&device_tag, &device_tag, &url).await
                .map_err(|e| AppError::Internal(format!("Failed to start stream: {}", e)))?;
            Ok(Json(ApiResponse::success(StartDeviceResult {
                success: true,
                message: format!("Stream started: {}", stream_info.play_url),
            })))
        }
    }
}

/// GET /api/v1/public/streams
pub async fn list_public_streams_handler(
    State(state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<Vec<Device>>>, AppError> {
    let devices = state.app.registry.device_service.list_public_devices().await;
    Ok(Json(ApiResponse::success(devices)))
}

// ═══════════════════════════════════════════════════════════════
// 分页响应类型
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Serialize)]
pub struct StartDeviceResult {
    pub success: bool,
    pub message: String,
}