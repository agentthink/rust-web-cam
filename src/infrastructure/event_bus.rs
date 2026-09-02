use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::{broadcast, Mutex, RwLock};
use crate::protocol::event::{SignalEvent, EventHandler, FnEventHandler};
use crate::domain::traits::EventPublisher;
use crate::error::AppError;

#[derive(Clone)]
pub struct EventBus {
    tx: Arc<broadcast::Sender<SignalEvent>>,
    handlers: Arc<DashMap<String, Vec<Arc<dyn EventHandler>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            tx: Arc::new(broadcast::channel(1024).0),
            handlers: Arc::new(DashMap::new()),
        }
    }

    pub async fn publish(&self, event: SignalEvent) -> anyhow::Result<()> {
        self.tx.send(event.clone()).map_err(|e| anyhow::anyhow!("broadcast send: {}", e))?;

        // 1. 发送到广播 channel（原有订阅者）
      //  let _ = self.tx.send(event.clone());

        // 2. 触发回调（新增）
        let event_type = event.event_type();

        // 触发精确匹配的回调
        if let Some(handlers) = self.handlers.get(&event_type) {
            for handler in handlers.value() {
                let handler = handler.clone();
                let event = event.clone();
                tokio::spawn(async move {
                    handler.handle(event).await;
                });
            }
        }

        // 触发 "*" 通配符回调（监听所有事件）
        if let Some(handlers) = self.handlers.get("*") {
            for handler in handlers.value() {
                let handler = handler.clone();
                let event = event.clone();
                tokio::spawn(async move {
                    handler.handle(event).await;
                });
            }
        }

        Ok(())
    }

    pub fn on(&self, event_type: &str, handler: Arc<dyn EventHandler>) {
     //   let handler = Arc::new(Mutex::new(handler));
        self.handlers
            .entry(event_type.to_string())
            .or_default()
            .push(handler);

    }
    pub fn on_fn<F, Fut>(&self, event_type: &str, f: F)
    where
        F: Fn(SignalEvent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.on(event_type, Arc::new(FnEventHandler::new(f)));
    }

    pub fn off(&self, event_type: &str) {
        self.handlers.remove(event_type);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SignalEvent> {
        self.tx.subscribe()
    }
    /// 订阅并在后台循环处理（便捷方法）
    pub fn subscribe_with_handler<F, Fut>(&self, mut f: F)
    where
        F: FnMut(SignalEvent) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut rx = self.tx.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => f(event).await,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("EventBus lagged {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl EventPublisher for EventBus {
    async fn publish(&self, event: SignalEvent) -> Result<(), AppError> {
        self.tx.send(event).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<SignalEvent> {
        self.tx.subscribe()
    }
}
