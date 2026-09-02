use std::sync::Arc;
use crate::context::ServiceRegistry;
use crate::auth::AuthState;

/// 完整应用状态（传递给 Axum）
#[derive(Clone)]
pub struct FullState {
    pub app: AppState,
    pub auth: AuthState,
}

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<ServiceRegistry>,
}

impl AppState {
    pub fn from_registry(registry: &Arc<ServiceRegistry>) -> Self {
        Self {
            registry: registry.clone(),
        }
    }
}