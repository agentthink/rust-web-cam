use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

/// Redis 消息总线
#[derive(Clone)]
pub struct RedisBus {
    url: String,
    conn: Arc<RwLock<Option<MultiplexedConnection>>>,
    max_retries: u32,
}

impl RedisBus {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let url = url.to_string();
        tracing::debug!("[Redis] Connecting to: {}", url);
        let conn = Self::create_connection(&url).await?;
        Ok(Self {
            url,
            conn: Arc::new(RwLock::new(Some(conn))),
            max_retries: 2,
        })
    }

    pub fn new_disconnected(url: &str) -> Self {
        let url = url.to_string();
        tracing::warn!("[Redis] Created disconnected bus for: {}", url);
        Self {
            url,
            conn: Arc::new(RwLock::new(None)),
            max_retries: 2,
        }
    }

    async fn create_connection(url: &str) -> anyhow::Result<MultiplexedConnection> {
        let client = Client::open(url)?;
        let conn = timeout(
            Duration::from_secs(5),
            client.get_multiplexed_async_connection(),
        )
            .await
            .map_err(|_| anyhow::anyhow!("Redis connection timeout"))?
            .map_err(|e| anyhow::anyhow!("Redis connection error: {}", e))?;
        Ok(conn)
    }

    pub async fn reconnect(&self) -> anyhow::Result<()> {
        tracing::debug!("[Redis] Reconnecting to: {}", self.url);
        let new_conn = Self::create_connection(&self.url).await?;
        let mut conn = self.conn.write().await;
        *conn = Some(new_conn);
        Ok(())
    }

    /// 带重试的执行器
    ///
    /// 使用 `Pin<Box<dyn Future>>` 消除生命周期问题。
    /// 闭包接收 `&mut MultiplexedConnection`，返回拥有所有权的 Future。
    async fn execute_with_retry<T, E>(
        &self,
        operation_name: &str,
        mut f: impl FnMut(&mut MultiplexedConnection) -> Pin<Box<dyn Future<Output = std::result::Result<T, E>> + Send + '_>>,
    ) -> anyhow::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
        T: Send,
    {
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 0..=self.max_retries {
            let mut conn_guard = self.conn.write().await;

            if let Some(ref mut conn) = *conn_guard {
                match f(conn).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        tracing::warn!(
                            "[Redis] {} attempt {}/{} failed: {}",
                            operation_name,
                            attempt + 1,
                            self.max_retries + 1,
                            e
                        );
                        last_error = Some(anyhow::anyhow!("{}", e));
                        *conn_guard = None;
                        drop(conn_guard);
                    }
                }
            } else {
                drop(conn_guard);
            }

            if attempt < self.max_retries {
                match self.reconnect().await {
                    Ok(()) => {
                        tracing::debug!("[Redis] Reconnected for {}", operation_name);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "Operation {} failed after {} retries",
                operation_name,
                self.max_retries + 1
            )
        }))
    }

    // ═══════════════════════════════════════════════════════════
    // 发布/订阅
    // ═══════════════════════════════════════════════════════════

    pub async fn publish<T: Serialize>(&self, channel: &str, message: &T) -> anyhow::Result<()> {
        let msg = serde_json::to_string(message)?;
        self.publish_str(channel, &msg).await
    }

    pub async fn publish_str(&self, channel: &str, msg: &str) -> anyhow::Result<()> {
        let channel = channel.to_string();
        let msg = msg.to_string();

        self.execute_with_retry("publish", move |conn| {
            let channel = channel.clone();
            let msg = msg.clone();
            Box::pin(async move {
                conn.publish::<_, _, ()>(&channel, msg).await
            })
        })
            .await
    }

    pub async fn subscribe(&self, _channel: &str) -> anyhow::Result<()> {
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════
    // 键值操作
    // ═══════════════════════════════════════════════════════════

    pub async fn set<T: Serialize>(
        &self, key: &str, value: &T, ttl_secs: Option<u64>,
    ) -> anyhow::Result<()> {
        let val = serde_json::to_string(value)?;
        self.set_json_str(key, &val, ttl_secs).await
    }

    pub async fn set_json_str(
        &self, key: &str, val: &str, ttl_secs: Option<u64>,
    ) -> anyhow::Result<()> {
        let key = key.to_string();
        let val = val.to_string();

        self.execute_with_retry("set", move |conn| {
            let key = key.clone();
            let val = val.clone();
            Box::pin(async move {
                match ttl_secs {
                    Some(ttl) => conn.set_ex::<_, _, ()>(&key, val, ttl).await,
                    None => conn.set::<_, _, ()>(&key, val).await,
                }
            })
        })
            .await
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> anyhow::Result<Option<T>> {
        match self.get_json_str(key).await? {
            Some(json) => {
                let value = serde_json::from_value(json)
                    .map_err(|e| anyhow::anyhow!("Deserialize error: {}", e))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub async fn get_json_str(&self, key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let key = key.to_string();

        let result: Option<String> = self
            .execute_with_retry("get", move |conn| {
                let key = key.clone();
                Box::pin(async move { conn.get::<_, Option<String>>(&key).await })
            })
            .await?;

        result
            .map(|s| serde_json::from_str(&s).map_err(|e| anyhow::anyhow!("JSON parse: {}", e)))
            .transpose()
    }

    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        let key = key.to_string();
        self.execute_with_retry("delete", move |conn| {
            let key = key.clone();
            Box::pin(async move { conn.del::<_, ()>(&key).await })
        })
            .await
    }

    pub async fn exists(&self, key: &str) -> anyhow::Result<bool> {
        let key = key.to_string();
        self.execute_with_retry("exists", move |conn| {
            let key = key.clone();
            Box::pin(async move { conn.exists::<_, bool>(&key).await })
        })
            .await
    }

    pub async fn incr(&self, key: &str) -> anyhow::Result<i64> {
        let key = key.to_string();
        self.execute_with_retry("incr", move |conn| {
            let key = key.clone();
            Box::pin(async move { conn.incr::<_, i64, i64>(&key, 1i64).await })
        })
            .await
    }

    // ═══════════════════════════════════════════════════════════
    // 哈希表操作
    // ═══════════════════════════════════════════════════════════

    pub async fn hset(&self, key: &str, field: &str, value: &str) -> anyhow::Result<()> {
        let key = key.to_string();
        let field = field.to_string();
        let value = value.to_string();

        self.execute_with_retry("hset", move |conn| {
            let key = key.clone();
            let field = field.clone();
            let value = value.clone();
            Box::pin(async move { conn.hset::<_, _, _, ()>(&key, &field, value).await })
        })
            .await
    }

    pub async fn hmset(&self, key: &str, fields: &[(&str, &str)]) -> anyhow::Result<()> {
        let key = key.to_string();
        let fields: Vec<(String, String)> = fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        self.execute_with_retry("hmset", move |conn| {
            let key = key.clone();
            let fields: Vec<(String, String)> = fields.clone();
            Box::pin(async move {
                let refs: Vec<(&str, &str)> = fields
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                conn.hset_multiple(&key, &refs).await
            })
        })
            .await
    }

    pub async fn hget(&self, key: &str, field: &str) -> anyhow::Result<Option<String>> {
        let key = key.to_string();
        let field = field.to_string();

        self.execute_with_retry("hget", move |conn| {
            let key = key.clone();
            let field = field.clone();
            Box::pin(async move { conn.hget::<_, _, Option<String>>(&key, &field).await })
        })
            .await
    }

    pub async fn hgetall(
        &self, key: &str,
    ) -> anyhow::Result<std::collections::HashMap<String, String>> {
        let key = key.to_string();
        self.execute_with_retry("hgetall", move |conn| {
            let key = key.clone();
            Box::pin(async move {
                conn.hgetall::<_, std::collections::HashMap<String, String>>(&key).await
            })
        })
            .await
    }

    // ═══════════════════════════════════════════════════════════
    // 过期时间
    // ═══════════════════════════════════════════════════════════

    pub async fn expire(&self, key: &str, seconds: u64) -> anyhow::Result<()> {
        let key = key.to_string();
        self.execute_with_retry("expire", move |conn| {
            let key = key.clone();
            Box::pin(async move { conn.expire::<_, ()>(&key, seconds as i64).await })
        })
            .await
    }

    pub async fn expire_key(&self, key: &str, seconds: u64) -> anyhow::Result<()> {
        self.expire(key, seconds).await
    }

    // ═══════════════════════════════════════════════════════════
    // 集合操作
    // ═══════════════════════════════════════════════════════════

    pub async fn sadd(&self, key: &str, member: &str) -> anyhow::Result<()> {
        let key = key.to_string();
        let member = member.to_string();

        self.execute_with_retry("sadd", move |conn| {
            let key = key.clone();
            let member = member.clone();
            Box::pin(async move { conn.sadd::<_, _, ()>(&key, member).await })
        })
            .await
    }

    pub async fn srem(&self, key: &str, member: &str) -> anyhow::Result<()> {
        let key = key.to_string();
        let member = member.to_string();

        self.execute_with_retry("srem", move |conn| {
            let key = key.clone();
            let member = member.clone();
            Box::pin(async move { conn.srem::<_, _, ()>(&key, member).await })
        })
            .await
    }

    pub async fn smembers(&self, key: &str) -> anyhow::Result<Vec<String>> {
        let key = key.to_string();
        self.execute_with_retry("smembers", move |conn| {
            let key = key.clone();
            Box::pin(async move { conn.smembers::<_, Vec<String>>(&key).await })
        })
            .await
    }
}