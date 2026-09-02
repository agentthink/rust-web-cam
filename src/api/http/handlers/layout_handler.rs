use std::sync::Arc;
use axum::{extract::State, extract::Path, Json};
use crate::api::state::FullState;
use crate::api::response::ApiResponse;
use crate::error::AppError;
use crate::domain::player_layout::{PlayerLayout, CreateLayoutRequest, UpdateLayoutRequest};

/// GET /api/v1/layouts
pub async fn list_layouts_handler(
    State(state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<Vec<PlayerLayout>>>, AppError> {
    let layouts = state.app.registry.player_layout_service.list().await?;
    Ok(Json(ApiResponse::success(layouts)))
}

/// GET /api/v1/layouts/:id
pub async fn get_layout_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<PlayerLayout>>, AppError> {
    let layout = state.app.registry.player_layout_service.get(id).await?
        .ok_or_else(|| AppError::NotFound(format!("Layout {} not found", id)))?;
    Ok(Json(ApiResponse::success(layout)))
}

/// POST /api/v1/layouts
pub async fn create_layout_handler(
    State(state): State<Arc<FullState>>,
    Json(req): Json<CreateLayoutRequest>,
) -> Result<Json<ApiResponse<PlayerLayout>>, AppError> {
    let layout = state.app.registry.player_layout_service.create(req).await?;
    Ok(Json(ApiResponse::success(layout)))
}

/// PUT /api/v1/layouts/:id
pub async fn update_layout_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLayoutRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.app.registry.player_layout_service.update(id, req).await?;
    Ok(Json(ApiResponse::success(())))
}

/// DELETE /api/v1/layouts/:id
pub async fn delete_layout_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.app.registry.player_layout_service.delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// PUT /api/v1/layouts/:id/default
pub async fn set_default_layout_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.app.registry.player_layout_service.set_default(id).await?;
    Ok(Json(ApiResponse::success(())))
}