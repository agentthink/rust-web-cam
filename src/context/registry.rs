use once_cell::sync::OnceCell;
use std::sync::Arc;
use crate::context::ServiceRegistry;

/// 全局 ServiceRegistry 实例
static GLOBAL_REGISTRY: OnceCell<Arc<ServiceRegistry>> = OnceCell::new();

/// 初始化全局 ServiceRegistry
pub fn init_registry(registry: Arc<ServiceRegistry>) {
    GLOBAL_REGISTRY
        .set(registry)
        .map_err(|_| tracing::warn!("[Registry] Already initialized"))
        .ok();
}

/// 获取全局 ServiceRegistry 引用
pub fn registry() -> Arc<ServiceRegistry> {
    GLOBAL_REGISTRY
        .get()
        .expect("ServiceRegistry not initialized")
        .clone()
}

/// 在闭包中使用 ServiceRegistry
pub fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&ServiceRegistry) -> R,
{
    let reg = registry();
    f(&reg)
}