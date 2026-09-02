pub mod routes;
pub mod handlers;

use std::sync::Arc;
use axum::Router;
use crate::api::state::{AppState, FullState};
use crate::auth::AuthState;

/// 创建 API 路由
pub fn create_router(state: AppState, auth_state: AuthState) -> Router {
    let full_state = FullState {
        app: state,
        auth: auth_state,
    };
    routes::create_routes(full_state)
}