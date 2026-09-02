use std::sync::Arc;
use axum::{extract::State, Json};
use crate::api::state::FullState;

pub async fn dashboard_handler(
    State(state): State<Arc<FullState>>,
) -> Json<serde_json::Value> {
    let data = state.app.registry.get_dashboard_data().await;
    Json(data)
}

pub async fn reload_cache_handler(
    State(state): State<Arc<FullState>>,
) -> Json<serde_json::Value> {
    match state.app.registry.infra.db.reload_devices_cache().await {
        Ok(count) => Json(serde_json::json!({
            "code": 0,
            "msg": "ok",
            "data": { "count": count }
        })),
        Err(e) => Json(serde_json::json!({
            "code": 1,
            "msg": e.to_string()
        })),
    }
}