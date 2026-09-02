use dashmap::DashMap;
use std::sync::Arc;
use crate::adapter::media_server::{MediaServerAdapter, ServerStatus};
use crate::config::{MediaServersConfig, MediaServerConfig};

pub struct ClusterManager {
    servers: Arc<DashMap<String, Arc<dyn MediaServerAdapter>>>,
    enabled: Arc<DashMap<String, bool>>,
    strategy: ClusterStrategy,
    round_robin: Arc<std::sync::atomic::AtomicU32>,
}

#[derive(Debug, Clone, Copy)]
pub enum ClusterStrategy {
    RoundRobin,
    LeastConnections,
    Random,
}

impl Default for ClusterStrategy {
    fn default() -> Self { ClusterStrategy::RoundRobin }
}

impl ClusterManager {
    pub fn new(config: MediaServersConfig) -> Self {
        Self::new_from_configs(config.servers)
    }

    pub fn new_from_configs(_configs: Vec<MediaServerConfig>) -> Self {
        let servers = Arc::new(DashMap::new());
        let enabled = Arc::new(DashMap::new());
        let strategy = ClusterStrategy::RoundRobin;
        let round_robin = Arc::new(std::sync::atomic::AtomicU32::new(0));

        Self { servers, enabled, strategy, round_robin }
    }

    fn create_adapter(config: &MediaServerConfig) -> Option<Arc<dyn MediaServerAdapter>> {
        match config.server_type.as_str() {
            "zlmediakit" => Some(Arc::new(crate::adapter::media_server::zlmediakit::ZlMediaKitAdapter::new(config.clone()))),
            "srs" => Some(Arc::new(crate::adapter::media_server::srs::SrsAdapter::new(config.clone()))),
            "xiu" => Some(Arc::new(crate::adapter::media_server::xiu::XiuAdapter::new(config.clone()))),
            _ => None,
        }
    }

    pub fn register_server(&self, config: MediaServerConfig) {
        if let Some(adapter) = Self::create_adapter(&config) {
            self.servers.insert(config.server_tag.clone(), adapter);
            self.enabled.insert(config.server_tag.clone(), config.enabled);
            tracing::info!("Registered media server: {} (tag={}, type={})", config.name, config.server_tag, config.server_type);
        }
    }

    pub fn unregister_server(&self, tag: &str) {
        self.servers.remove(tag);
        self.enabled.remove(tag);
        tracing::info!("Unregistered media server: {}", tag);
    }

    pub async fn get_server_status(&self, tag: &str) -> Option<ServerStatus> {
        let adapter = match self.servers.get(tag) {
            Some(a) => a.value().clone(),
            None => {
                tracing::warn!("[ClusterManager] get_server_status: tag '{}' not found in cluster", tag);
                return None;
            }
        };
        match adapter.get_status().await {
            Ok(status) => Some(status),
            Err(e) => {
                tracing::warn!("[ClusterManager] get_server_status: tag '{}' get_status failed: {}", tag, e);
                None
            }
        }
    }

    pub async fn is_stream_online(&self, app: &str, stream_key: &str, server_tag: &str) -> bool {
        if let Some(adapter) = self.servers.get(server_tag) {
            match adapter.value().is_stream_online(app, stream_key).await {
                Ok(online) => return online,
                Err(e) => tracing::warn!("[ClusterManager] is_stream_online failed for {}: {}", stream_key, e),
            }
        }
        false
    }

    pub async fn select_any_server(&self) -> Option<Arc<dyn MediaServerAdapter>> {
        let candidates: Vec<_> = self.servers.iter()
            .filter(|entry| self.enabled.get(entry.key()).map(|e| *e.value()).unwrap_or(false))
            .map(|e| e.value().clone())
            .collect();

        if candidates.is_empty() { return None; }

        match self.strategy {
            ClusterStrategy::RoundRobin => {
                let idx = self.round_robin.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as usize % candidates.len();
                Some(candidates[idx].clone())
            }
            ClusterStrategy::LeastConnections => {
                let mut counts: Vec<_> = futures::future::join_all(
                    candidates.iter().map(|s| async {
                        let count = s.get_session_count().await.unwrap_or(u32::MAX);
                        (Arc::clone(s), count)
                    })
                ).await;
                counts.sort_by(|a, b| a.1.cmp(&b.1));
                counts.into_iter().next().map(|(s, _)| s)
            }
            ClusterStrategy::Random => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as usize;
                Some(candidates[seed % candidates.len()].clone())
            }
        }
    }

    pub async fn select_server(&self) -> Option<Arc<dyn MediaServerAdapter>> {
        let candidates: Vec<_> = futures::future::join_all(
            self.servers.iter()
                .filter(|entry| self.enabled.get(entry.key()).map(|e| *e.value()).unwrap_or(false))
                .map(|e| e.value().clone())
                .map(|s| async move {
                    if s.is_online().await { Some(s) } else { None }
                })
        ).await.into_iter().flatten().collect();

        if candidates.is_empty() { return None; }

        match self.strategy {
            ClusterStrategy::RoundRobin => {
                let idx = self.round_robin.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as usize % candidates.len();
                Some(candidates[idx].clone())
            }
            _ => Some(candidates[0].clone()),
        }
    }

    pub fn get_all_servers(&self) -> Vec<Arc<dyn MediaServerAdapter>> {
        self.servers.iter().map(|s| s.value().clone()).collect()
    }

    pub fn server_count(&self) -> usize { self.servers.len() }

    pub fn get_server(&self, tag: &str) -> Option<Arc<dyn MediaServerAdapter>> {
        self.servers.get(tag).map(|s| s.value().clone())
    }

    pub fn set_server_enabled(&self, tag: &str, enabled: bool) -> bool {
        if self.servers.contains_key(tag) {
            self.enabled.insert(tag.to_string(), enabled);
            return true;
        }
        false
    }

    pub fn get_all_server_tags(&self) -> Vec<String> {
        self.servers.iter().map(|e| e.key().clone()).collect()
    }
}