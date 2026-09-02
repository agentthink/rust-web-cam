use std::sync::Arc;
use axum::{extract::State, extract::Query, Json};
use serde::Deserialize;
use crate::api::response::ApiResponse;
use crate::api::http::handlers::device_handler::PaginatedResponse;
use crate::api::state::FullState;
use crate::error::AppError;
use crate::domain::Session;

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl PaginationQuery {
    pub fn limit(&self) -> usize { self.limit.unwrap_or(50).min(500) }
    pub fn offset(&self) -> usize { self.offset.unwrap_or(0) }
}

/// GET /api/v1/sessions
pub async fn list_sessions_handler(
    State(state): State<Arc<FullState>>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<Session>>>, AppError> {
    let registry = &state.app.registry;
    let sessions = registry.session_service.list_sessions_paginated(q.limit(), q.offset()).await;
    let total = registry.session_service.count_sessions().await;

    Ok(Json(ApiResponse::success(PaginatedResponse {
        items: sessions,
        total,
        limit: q.limit(),
        offset: q.offset(),
    })))
}