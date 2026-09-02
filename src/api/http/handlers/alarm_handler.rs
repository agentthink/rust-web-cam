use std::sync::Arc;
use axum::{
    extract::{Path, State, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::api::state::FullState;
use crate::api::response::ApiResponse;
use crate::error::AppError;
use crate::domain::alarm::{Alarm, CreateAlarmRequest};

#[derive(Debug, Deserialize)]
pub struct AlarmQuery {
    pub device_id: Option<i64>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

impl AlarmQuery {
    pub fn limit(&self) -> usize { self.page_size.unwrap_or(20).min(100) }
    pub fn offset(&self) -> usize { self.page.unwrap_or(1).saturating_sub(1) * self.limit() }
}

#[derive(Debug, Serialize)]
pub struct PaginatedAlarms {
    pub items: Vec<Alarm>,
    pub total: i64,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct MarkProcessedRequest {
    pub processed: bool,
}

pub async fn list_alarms_handler(
    State(state): State<Arc<FullState>>,
    Query(q): Query<AlarmQuery>,
) -> Result<Json<ApiResponse<PaginatedAlarms>>, AppError> {
    let registry = &state.app.registry;
    let alarms = registry.infra.db.list_alarms(q.device_id, q.limit(), q.offset()).await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
    let total = registry.infra.db.count_alarms(q.device_id).await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

    Ok(Json(ApiResponse::success(PaginatedAlarms {
        items: alarms,
        total,
        page: q.page.unwrap_or(1),
        page_size: q.limit(),
    })))
}

pub async fn mark_alarm_processed_handler(
    State(state): State<Arc<FullState>>,
    Path(id): Path<i64>,
    Json(req): Json<MarkProcessedRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let registry = &state.app.registry;
    registry.infra.db.mark_alarm_processed(id, req.processed).await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
    Ok(Json(ApiResponse::success(())))
}