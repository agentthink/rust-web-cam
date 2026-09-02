use std::sync::Arc;
use axum::{
    extract::{Path, State},  // ✅ Query 在这里
    Extension,
    Json,
};
use axum::extract::Query;
use serde::Deserialize;
use crate::api::state::FullState;
use crate::api::response::ApiResponse;
use crate::error::AppError;
use crate::domain::region::{Region, RegionNode};

#[derive(Debug, Deserialize)]
pub struct RegionQuery {
    pub parent: Option<String>,
}

/// GET /api/v1/regions
pub async fn list_regions_handler(
    State(state): State<Arc<FullState>>,
    Query(q): Query<RegionQuery>,
) -> Result<Json<ApiResponse<Vec<Region>>>, AppError> {
    let regions = if let Some(parent) = &q.parent {
        state.app.registry.device_service.list_region_children(parent).await
    } else {
        state.app.registry.device_service.list_region_children("").await
    };
    Ok(Json(ApiResponse::success(regions)))
}

/// GET /api/v1/regions/tree
pub async fn list_region_tree_handler(
    State(state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<Vec<RegionNode>>>, AppError> {
    let tree = state.app.registry.device_service.get_region_tree().await;
    Ok(Json(ApiResponse::success(tree)))
}