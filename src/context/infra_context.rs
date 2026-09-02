use std::sync::Arc;
use tokio::sync::broadcast;
use crate::config::AppConfig;
use crate::infrastructure::{DbRepository, RedisStore, EventBus};
use crate::application::ws_broadcaster::WsBroadcaster;
use crate::monitoring::Metrics;
use crate::protocol::event::SignalEvent;

/// 基础设施上下文
#[derive(Clone)]
pub struct InfraContext {
    pub db: Arc<DbRepository>,
    pub redis: Arc<RedisStore>,
    pub event_bus: Arc<EventBus>,
    pub metrics: Arc<Metrics>,
    pub ws_broadcaster: Arc<WsBroadcaster>,
    pub config: Arc<AppConfig>,
}

impl InfraContext {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        tracing::info!("[InfraContext] Initializing infrastructure...");

        let db = Arc::new(
            DbRepository::new(&config.database.url, config.database.debug_sql.unwrap_or(false))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?
        );

        let redis = match RedisStore::new(&config.redis.url).await {
            Ok(r) => Arc::new(r),
            Err(e) => {
                tracing::warn!("[InfraContext] Redis connection failed (will retry later): {}", e);
                Arc::new(RedisStore::new_placeholder(&config.redis.url))
            }
        };

        let event_bus = Arc::new(EventBus::new());
        let metrics = Arc::new(Metrics::new());
        let ws_broadcaster = Arc::new(WsBroadcaster::new());
        let config = Arc::new(config);

        tracing::info!(
            "[InfraContext] Initialized: db={}, redis={}",
            config.database.url.split('@').last().unwrap_or("***"),
            config.redis.url
        );

        Ok(Self {
            db,
            redis,
            event_bus,
            metrics,
            ws_broadcaster,
            config,
        })
    }

    pub async fn from_parts(db: Arc<DbRepository>, config: Arc<AppConfig>) -> anyhow::Result<Self> {
        tracing::info!("[InfraContext] Building from parts...");

        let redis = match RedisStore::new(&config.redis.url).await {
            Ok(r) => Arc::new(r),
            Err(e) => {
                tracing::warn!("[InfraContext] Redis connection failed (will retry later): {}", e);
                Arc::new(RedisStore::new_placeholder(&config.redis.url))
            }
        };

        let event_bus = Arc::new(EventBus::new());
        let metrics = Arc::new(Metrics::new());
        let ws_broadcaster = Arc::new(WsBroadcaster::new());

        tracing::info!(
            "[InfraContext] Initialized: db={}, redis={}",
            config.database.url.split('@').last().unwrap_or("***"),
            config.redis.url
        );

        Ok(Self {
            db,
            redis,
            event_bus,
            metrics,
            ws_broadcaster,
            config,
        })
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<SignalEvent> {
        self.event_bus.subscribe()
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn ws(&self) -> &WsBroadcaster {
        &self.ws_broadcaster
    }

    pub async fn publish_event(&self, event: SignalEvent) -> anyhow::Result<()> {
        self.metrics.event_published();
        self.event_bus.publish(event).await
    }
}

/// 基础设施上下文构建器（用于测试）
///
/// 仅在 `#[cfg(test)]` 下可用。
/// 允许逐步注入依赖，未注入的依赖使用默认实现。
#[cfg(test)]
pub struct InfraContextBuilder {
    db: Option<Arc<DbRepository>>,
    redis: Option<Arc<RedisStore>>,
    event_bus: Option<Arc<EventBus>>,
    metrics: Option<Arc<Metrics>>,
    ws_broadcaster: Option<Arc<WsBroadcaster>>,
    config: Option<Arc<AppConfig>>,
}

#[cfg(test)]
impl InfraContextBuilder {
    pub fn new() -> Self {
        Self {
            db: None,
            redis: None,
            event_bus: None,
            metrics: None,
            ws_broadcaster: None,
            config: None,
        }
    }

    pub fn with_db(mut self, db: Arc<DbRepository>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_redis(mut self, redis: Arc<RedisStore>) -> Self {
        self.redis = Some(redis);
        self
    }

    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub fn build(self) -> InfraContext {
        InfraContext {
            db: self.db.unwrap_or_else(|| {
                let db_url = std::env::var("TEST_DATABASE_URL")
                    .unwrap_or_else(|_| "postgres://postgres:test@localhost:5434/rustcam_test".to_string());

                let rt = tokio::runtime::Runtime::new().expect("Failed to create test runtime");
                rt.block_on(async {
                    DbRepository::new(&db_url, false).await.expect("Failed to create test DbRepository")
                })
                    .into()
            }),
            redis: self.redis.unwrap_or_else(|| {
                let redis_url = std::env::var("TEST_REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379/15".to_string());

                let rt = tokio::runtime::Runtime::new().expect("Failed to create test runtime");
                rt.block_on(async {
                    RedisStore::new(&redis_url).await.expect("Failed to create test RedisStore")
                })
                    .into()
            }),
            event_bus: self.event_bus.unwrap_or_else(|| Arc::new(EventBus::new())),
            metrics: self.metrics.unwrap_or_else(|| Arc::new(Metrics::new())),
            ws_broadcaster: self.ws_broadcaster.unwrap_or_else(|| Arc::new(WsBroadcaster::new())),
            config: self.config.unwrap_or_else(|| Arc::new(AppConfig::default())),
        }
    }
}