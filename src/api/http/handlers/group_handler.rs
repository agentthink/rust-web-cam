use std::sync::Arc;
use axum::{extract::State, extract::Path, Json};
use crate::api::state::FullState;
use crate::api::response::ApiResponse;
use crate::error::AppError;
use crate::domain::device_group::{
    DeviceGroup, DeviceGroupNode, CreateGroupRequest, UpdateGroupRequest, AssignGroupRequest,
};

/// GET /api/v1/groups/tree
pub async fn list_group_tree_handler(
    State(state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<Vec<DeviceGroupNode>>>, AppError> {
    let tree = state.app.registry.device_service.get_group_tree().await;
    Ok(Json(ApiResponse::success(tree)))
}

/// GET /api/v1/groups
pub async fn list_groups_handler(
    State(state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<Vec<DeviceGroup>>>, AppError> {
    let groups = state.app.registry.device_service.list_groups().await;
    Ok(Json(ApiResponse::success(groups)))
}

/// POST /api/v1/groups
pub async fn create_group_handler(
    State(state): State<Arc<FullState>>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<ApiResponse<i64>>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("Group name cannot be empty".to_string()));
    }
    let id = state.app.registry.device_service.create_group(req).await?;
    Ok(Json(ApiResponse::success(id)))
}

/// PUT /api/v1/groups/:id
pub async fn update_group_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.app.registry.device_service.update_group(id, req).await?;
    Ok(Json(ApiResponse::success(())))
}

/// DELETE /api/v1/groups/:id
pub async fn delete_group_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.app.registry.device_service.delete_group(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// PUT /api/v1/devices/:id/group
pub async fn assign_device_group_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
    Json(req): Json<AssignGroupRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.app.registry.device_service.assign_device_group(id, req.group_id).await?;
    Ok(Json(ApiResponse::success(())))
}