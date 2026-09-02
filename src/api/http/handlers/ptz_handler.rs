use std::sync::Arc;
use axum::{
    extract::{Path, State},
    Json,
};
use crate::api::state::FullState;
use crate::api::response::ApiResponse;
use crate::error::AppError;
use crate::domain::ptz::{PtzControlRequest, PtzMoveType, PtzPreset, PtzStatus, PtzControlResult};
use crate::protocol::event::PtzCommand;

fn parse_simple_command(cmd: &str) -> Option<PtzCommand> {
    match cmd.trim().to_lowercase().as_str() {
        "up" => Some(PtzCommand::Up),
        "down" => Some(PtzCommand::Down),
        "left" => Some(PtzCommand::Left),
        "right" => Some(PtzCommand::Right),
        "zoom_in" | "zoomin" => Some(PtzCommand::ZoomIn),
        "zoom_out" | "zoomout" => Some(PtzCommand::ZoomOut),
        "focus_in" | "focusin" => Some(PtzCommand::FocusIn),
        "focus_out" | "focusout" => Some(PtzCommand::FocusOut),
        "stop" => Some(PtzCommand::Stop),
        _ => None,
    }
}

fn build_command(req: &PtzControlRequest) -> Result<PtzCommand, AppError> {
    if let Some(move_type) = &req.move_type {
        let pan = req.pan.unwrap_or(0.0);
        let tilt = req.tilt.unwrap_or(0.0);
        let zoom = req.zoom.unwrap_or(0.0);
        match move_type {
            PtzMoveType::Continuous => {
                let speed = req.speed.unwrap_or(50) as f64 / 100.0;
                Ok(PtzCommand::ContinuousMove {
                    pan: pan * speed,
                    tilt: tilt * speed,
                    zoom: zoom * speed,
                })
            }
            PtzMoveType::Absolute => {
                Ok(PtzCommand::AbsoluteMove { pan, tilt, zoom })
            }
            PtzMoveType::Relative => {
                Ok(PtzCommand::RelativeMove { pan, tilt, zoom })
            }
            PtzMoveType::GotoPreset => {
                let token = req.preset_token.clone()
                    .ok_or_else(|| AppError::BadRequest("Missing preset_token for GotoPreset".to_string()))?;
                Ok(PtzCommand::GotoPreset { preset_token: token })
            }
            PtzMoveType::SetPreset => {
                Ok(PtzCommand::SetPreset { preset_name: req.preset_name.clone() })
            }
            PtzMoveType::RemovePreset => {
                let token = req.preset_token.clone()
                    .ok_or_else(|| AppError::BadRequest("Missing preset_token for RemovePreset".to_string()))?;
                Ok(PtzCommand::RemovePreset { preset_token: token })
            }
            PtzMoveType::Stop => Ok(PtzCommand::Stop),
        }
    } else if let Some(cmd) = &req.command {
        parse_simple_command(cmd).ok_or_else(|| {
            AppError::BadRequest(format!("Unknown PTZ command: {}", cmd))
        })
    } else {
        Err(AppError::BadRequest("Missing field: command or move_type required".to_string()))
    }
}

/// POST /api/v1/devices/:id/ptz
pub async fn ptz_control_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
    Json(req): Json<PtzControlRequest>,
) -> Result<Json<ApiResponse<PtzControlResult>>, AppError> {
    let command = build_command(&req)?;

    let device_service = &state.app.registry.device_service;

    match device_service.handle_ptz_control(id, command.clone(), req.speed).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            tracing::warn!("[PTZ] Control failed: device={} err={}", id, e);
            Err(e)
        }
    }
}

/// GET /api/v1/devices/:id/ptz/presets
pub async fn get_ptz_presets_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<PtzPreset>>>, AppError> {
    let presets = state.app.registry.device_service.list_ptz_presets(id).await?;
    Ok(Json(ApiResponse::success(presets)))
}

/// POST /api/v1/devices/:id/ptz/presets
pub async fn create_ptz_preset_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
    Json(req): Json<CreatePresetRequest>,
) -> Result<Json<ApiResponse<PtzPreset>>, AppError> {
    let preset = state.app.registry.device_service.create_ptz_preset(id, &req.name).await?;
    Ok(Json(ApiResponse::success(preset)))
}

/// DELETE /api/v1/devices/:id/ptz/presets/:token
pub async fn delete_ptz_preset_handler(
    State(state): State<Arc<FullState>>,
    Path((id, token)): Path<(i64, String)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.app.registry.device_service.delete_ptz_preset(id, &token).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/devices/:id/ptz/status
pub async fn get_ptz_status_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<PtzStatus>>, AppError> {
    let status = state.app.registry.device_service.get_ptz_status(id).await?;
    Ok(Json(ApiResponse::success(status)))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreatePresetRequest {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct RenamePresetRequest {
    pub name: String,
}

/// PUT /api/v1/devices/:id/ptz/presets/:token
pub async fn rename_ptz_preset_handler(
    State(state): State<Arc<FullState>>,
    Path((id, token)): Path<(i64, String)>,
    Json(req): Json<RenamePresetRequest>,
) -> Result<Json<ApiResponse<PtzPreset>>, AppError> {
    let preset = state.app.registry.device_service.rename_ptz_preset(id, &token, &req.name).await?;
    Ok(Json(ApiResponse::success(preset)))
}

// ─── Channel-level PTZ handlers ─────────────────────────────────────────────

/// POST /api/v1/channels/{device_tag}/{channel_tag}/ptz
pub async fn channel_ptz_control_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
    Json(req): Json<PtzControlRequest>,
) -> Result<Json<ApiResponse<PtzControlResult>>, AppError> {
    let command = build_command(&req)?;
    
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    
    match state.app.registry.device_service.handle_ptz_control(device.id, command.clone(), req.speed).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            tracing::warn!("[PTZ] Channel control failed: {}/{} err={}", device_tag, channel_tag, e);
            Err(e)
        }
    }
}

/// GET /api/v1/channels/{device_tag}/{channel_tag}/ptz/presets
pub async fn get_channel_ptz_presets_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Vec<PtzPreset>>>, AppError> {
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    let presets = state.app.registry.device_service.list_ptz_presets(device.id).await?;
    Ok(Json(ApiResponse::success(presets)))
}

/// POST /api/v1/channels/{device_tag}/{channel_tag}/ptz/presets
pub async fn create_channel_ptz_preset_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
    Json(req): Json<CreatePresetRequest>,
) -> Result<Json<ApiResponse<PtzPreset>>, AppError> {
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    let preset = state.app.registry.device_service.create_ptz_preset(device.id, &req.name).await?;
    Ok(Json(ApiResponse::success(preset)))
}

/// DELETE /api/v1/channels/{device_tag}/{channel_tag}/ptz/presets/:token
pub async fn delete_channel_ptz_preset_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag, token)): Path<(String, String, String)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    state.app.registry.device_service.delete_ptz_preset(device.id, &token).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/channels/{device_tag}/{channel_tag}/ptz/status
pub async fn get_channel_ptz_status_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag)): Path<(String, String)>,
) -> Result<Json<ApiResponse<PtzStatus>>, AppError> {
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    let status = state.app.registry.device_service.get_ptz_status(device.id).await?;
    Ok(Json(ApiResponse::success(status)))
}

/// PUT /api/v1/channels/{device_tag}/{channel_tag}/ptz/presets/:token
pub async fn rename_channel_ptz_preset_handler(
    State(state): State<Arc<FullState>>,
    Path((device_tag, channel_tag, token)): Path<(String, String, String)>,
    Json(req): Json<RenamePresetRequest>,
) -> Result<Json<ApiResponse<PtzPreset>>, AppError> {
    let device = state.app.registry.device_service.get_device_by_device_tag(&device_tag)?
        .ok_or_else(|| AppError::NotFound(format!("Device {} not found", device_tag)))?;
    let preset = state.app.registry.device_service.rename_ptz_preset(device.id, &token, &req.name).await?;
    Ok(Json(ApiResponse::success(preset)))
}
