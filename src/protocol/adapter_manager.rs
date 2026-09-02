use crate::protocol::adapter::SignalAdapter;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type AdapterEntry = Arc<Mutex<Box<dyn SignalAdapter + Send + Sync>>>;

static ADAPTER_MAP: std::sync::LazyLock<
    std::sync::RwLock<std::collections::HashMap<String, AdapterEntry>>,
    fn() -> std::sync::RwLock<std::collections::HashMap<String, AdapterEntry>>,
> = std::sync::LazyLock::new(|| std::sync::RwLock::new(std::collections::HashMap::new()));

#[inline]
pub fn get_adapter(key: &str) -> Option<AdapterEntry> {
    ADAPTER_MAP.read().unwrap().get(key).cloned()
}

#[inline]
pub fn set_adapter(key: String, adapter: AdapterEntry) {
    ADAPTER_MAP.write().unwrap().insert(key, adapter);
}

#[inline]
pub fn remove_adapter(key: &str) {
    ADAPTER_MAP.write().unwrap().remove(key);
}

#[inline]
pub fn clear_adapters() {
    ADAPTER_MAP.write().unwrap().clear();
}

pub async fn cleanup_expired_subscriptions_all() -> usize {
    let mut total = 0;
    let keys: Vec<String> = ADAPTER_MAP.read().unwrap().keys().cloned().collect();
    for key in keys {
        if let Some(adapter) = get_adapter(&key) {
            let guard = adapter.lock().await;
            total += guard.cleanup_expired_subscriptions().await;
        }
    }
    total
}
