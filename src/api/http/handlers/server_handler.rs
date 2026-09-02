use std::sync::Arc;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::api::state::FullState;
use crate::api::response::ApiResponse;
use crate::error::AppError;
use crate::domain::server::{ServerType, ServerProtocolPorts};

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub server_type: String,
    pub weight: Option<u32>,
    pub server_tag: String,
    #[serde(default)]
    pub protocol_ports: Option<ServerProtocolPorts>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServerRequest {
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub server_type: String,
    pub weight: u32,
    pub enabled: bool,
    pub server_tag: String,
    #[serde(default)]
    pub protocol_ports: Option<ServerProtocolPorts>,
}

/// GET /api/v1/servers
pub async fn list_servers_handler(
    State(state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let registry = &state.app.registry;
    let servers = registry.media_server_service.list();
    let mut result = Vec::new();

        for server in servers {
            let status = registry.media.cluster.get_server_status(&server.server_tag).await;
            result.push(serde_json::json!({
                "id": server.id,
                "name": server.name,
                "url": server.url,
                "api_key": server.api_key,
                "server_type": server.server_type,
                "server_tag": server.server_tag,
                "weight": server.weight,
                "enabled": server.enabled,
                "online": status.as_ref().map(|s| s.online).unwrap_or(false),
                "session_count": status.as_ref().map(|s| s.session_count).unwrap_or(0),
                "cpu_usage": status.as_ref().map(|s| s.cpu_usage).unwrap_or(0.0),
                "memory_usage": status.as_ref().map(|s| s.memory_usage).unwrap_or(0.0),
                "protocol_ports": server.protocol_ports,
            }));
        }

        Ok(Json(ApiResponse::success(result)))
    }

    /// POST /api/v1/servers
    pub async fn create_server_handler(
        State(state): State<Arc<FullState>>,
        Json(req): Json<CreateServerRequest>,
    ) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
        let registry = &state.app.registry;

        if req.name.trim().is_empty() {
            return Err(AppError::BadRequest("Server name cannot be empty".to_string()));
        }

        let server_type = ServerType::from_str(&req.server_type)
            .ok_or_else(|| AppError::BadRequest(format!("Invalid server_type: {}", req.server_type)))?;

        let weight = req.weight.unwrap_or(100);
        let protocol_ports = req.protocol_ports.clone().unwrap_or_default();
        let server_tag = req.server_tag.trim().to_string();

        let server = registry.media_server_service.create(
            req.name, req.url, req.api_key, server_type, weight, server_tag, protocol_ports,
        ).await;

        registry.media.cluster.register_server(server.to_config());

        if server.enabled {
            registry.stream_recovery_service.mark_streams_recovering_for_media_server(&server.server_tag).await;
            registry.stream_recovery_service.recover_streams_for_media_server(&server.server_tag).await;
        }

        Ok(Json(ApiResponse::success(serde_json::json!({
            "id": server.id,
            "name": server.name,
            "url": server.url,
            "api_key": server.api_key,
            "server_type": server.server_type,
            "server_tag": server.server_tag,
            "weight": server.weight,
            "enabled": server.enabled,
            "created_at": server.created_at,
        }))))
    }

    /// GET /api/v1/servers/:tag
pub async fn get_server_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let registry = &state.app.registry;
    let server = registry.media_server_service.get(&tag)
        .ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;

    let status = registry.media.cluster.get_server_status(&server.server_tag).await;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": server.id,
        "name": server.name,
        "url": server.url,
        "api_key": server.api_key,
        "server_type": server.server_type,
        "server_tag": server.server_tag,
        "weight": server.weight,
        "enabled": server.enabled,
        "online": status.as_ref().map(|s| s.online).unwrap_or(false),
        "session_count": status.as_ref().map(|s| s.session_count).unwrap_or(0),
        "cpu_usage": status.as_ref().map(|s| s.cpu_usage).unwrap_or(0.0),
        "memory_usage": status.as_ref().map(|s| s.memory_usage).unwrap_or(0.0),
        "protocol_ports": server.protocol_ports,
    }))))
}

/// PUT /api/v1/servers/:tag
pub async fn update_server_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
    Json(req): Json<UpdateServerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let registry = &state.app.registry;
    let old = registry.media_server_service.get(&tag)
        .ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;

    let server_type = ServerType::from_str(&req.server_type)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid server_type: {}", req.server_type)))?;

    let server = registry.media_server_service.update(
        &tag, req.name, req.url, req.api_key, server_type, req.weight,
        req.server_tag.trim().to_string(), req.protocol_ports.unwrap_or_default(),
    ).await.ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;

    registry.media.cluster.unregister_server(&old.name);
    registry.media.cluster.register_server(server.to_config());

    if server.enabled {
        registry.stream_recovery_service.mark_streams_recovering_for_media_server(&server.server_tag).await;
        registry.stream_recovery_service.recover_streams_for_media_server(&server.server_tag).await;
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": server.id,
        "name": server.name,
        "url": server.url,
        "api_key": server.api_key,
        "server_type": server.server_type,
        "server_tag": server.server_tag,
        "enabled": server.enabled,
    }))))
}

/// DELETE /api/v1/servers/:tag
pub async fn delete_server_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let registry = &state.app.registry;
    let server = registry.media_server_service.get(&tag)
        .ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;

    registry.media.cluster.unregister_server(&server.name);
    registry.media_server_service.delete(&tag).await;

    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/servers/:tag/status
pub async fn server_status_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let registry = &state.app.registry;
    let server = registry.media_server_service.get(&tag)
        .ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;

    let status = registry.media.cluster.get_server_status(&server.server_tag).await;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "online": status.as_ref().map(|s| s.online).unwrap_or(false),
        "session_count": status.as_ref().map(|s| s.session_count).unwrap_or(0),
        "cpu_usage": status.as_ref().map(|s| s.cpu_usage).unwrap_or(0.0),
        "memory_usage": status.as_ref().map(|s| s.memory_usage).unwrap_or(0.0),
    }))))
}

/// POST /api/v1/servers/:tag/refresh
pub async fn refresh_server_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let registry = &state.app.registry;
    let server = registry.media_server_service.get(&tag)
        .ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;

    let status = registry.media.cluster.get_server_status(&server.server_tag).await;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "online": status.as_ref().map(|s| s.online).unwrap_or(false),
        "session_count": status.as_ref().map(|s| s.session_count).unwrap_or(0),
    }))))
}

/// POST /api/v1/servers/:tag/enable
pub async fn enable_server_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let registry = &state.app.registry;
    let updated = registry.media_server_service.update_enabled(&tag, true)
        .ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;
    registry.media.cluster.set_server_enabled(&updated.name, true);

    registry.stream_recovery_service.mark_streams_recovering_for_media_server(&updated.server_tag).await;
    registry.stream_recovery_service.recover_streams_for_media_server(&updated.server_tag).await;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": updated.id, "name": updated.name, "enabled": updated.enabled,
    }))))
}

/// POST /api/v1/servers/:tag/disable
pub async fn disable_server_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let registry = &state.app.registry;
    let updated = registry.media_server_service.update_enabled(&tag, false)
        .ok_or_else(|| AppError::NotFound(format!("Server {} not found", tag)))?;
    registry.media.cluster.set_server_enabled(&updated.name, false);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": updated.id, "name": updated.name, "enabled": updated.enabled,
    }))))
}

/// GET /api/v1/servers/:tag/sessions
pub async fn server_sessions_handler(
    State(state): State<Arc<FullState>>,
    Path(tag): Path<String>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let registry = &state.app.registry;
    if let Some(adapter) = registry.media.cluster.get_server(&tag) {
        let sessions = adapter.get_sessions().await.unwrap_or_default();
        return Ok(Json(ApiResponse::success(sessions)));
    }
    Ok(Json(ApiResponse::success(vec![])))
}