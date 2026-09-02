// 在现有 RedisStore 实现基础上添加:

use async_trait::async_trait;
use crate::domain::traits::{CacheStore, CacheStoreExt};
use crate::adapter::media_server::{ServerStatus, StreamInfo};
use crate::adapter::message_bus::RedisBus;
use crate::error::AppError;
#[derive(Clone)]
pub struct RedisStore {
    bus: RedisBus,
}

impl RedisStore {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let bus = RedisBus::new(url).await?;
        Ok(Self { bus })
    }

    pub fn new_placeholder(url: &str) -> Self {
        Self {
            bus: RedisBus::new_disconnected(url),
        }
    }

    #[cfg(test)]
    pub fn new_mock() -> Self {
        // Mock 实现用于测试
        panic!("Use a proper Redis mock for tests")
    }

    pub async fn reconnect_redis(&self) -> anyhow::Result<()> {
        self.bus.reconnect().await
    }

    pub async fn expire_key(&self, key: &str, seconds: u64) -> Result<(), AppError> {
        self.bus.expire_key(key, seconds).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_server_status(&self, name: &str) -> anyhow::Result<Option<ServerStatus>> {
        let key = format!("media_servers:{}", name);
        let fields = self.bus.hgetall(&key).await?;
        if fields.is_empty() {
            return Ok(None);
        }

        let parse = |f: &str| -> f64 {
            fields.get(f).and_then(|v| v.parse().ok()).unwrap_or(0.0)
        };
        let parse_u64 = |f: &str| -> u64 {
            fields.get(f).and_then(|v| v.parse().ok()).unwrap_or(0)
        };
        let parse_bool = |f: &str| -> bool {
            fields.get(f).map(|v| v == "true").unwrap_or(false)
        };

        Ok(Some(ServerStatus {
            name: fields.get("name").cloned().unwrap_or_default(),
            server_type: fields.get("type").cloned().unwrap_or_default(),
            online: parse_bool("online"),
            session_count: parse_u64("session_count") as u32,
            cpu_usage: parse("cpu_usage"),
            memory_usage: parse("memory_usage"),
            bandwidth_in: parse_u64("bandwidth_in"),
            bandwidth_out: parse_u64("bandwidth_out"),
            last_heartbeat: fields.get("last_heartbeat").and_then(|v| v.parse().ok()),
        }))
    }
}

#[async_trait]
impl CacheStore for RedisStore {
    async fn set_json(
        &self, key: &str, value: &serde_json::Value, ttl_secs: Option<u64>,
    ) -> Result<(), AppError> {
        let val = serde_json::to_string(value)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        self.bus.set_json_str(key, &val, ttl_secs).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn get_json(&self, key: &str) -> Result<Option<serde_json::Value>, AppError> {
        self.bus.get_json_str(key).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn del(&self, key: &str) -> Result<(), AppError> {
        self.bus.delete(key).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn set_stream_info(&self, stream_key: &str, info: &StreamInfo) -> Result<(), AppError> {
        let key = format!("stream:{}", stream_key);
        let json = serde_json::to_value(info)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        self.set_json(&key, &json, Some(86400)).await
    }

    async fn get_stream_info(&self, stream_key: &str) -> Result<Option<StreamInfo>, AppError> {
        let key = format!("stream:{}", stream_key);
        self.get::<StreamInfo>(&key).await
    }

    async fn delete_stream_info(&self, stream_key: &str) -> Result<(), AppError> {
        let key = format!("stream:{}", stream_key);
        self.del(&key).await
    }

    async fn publish_event_json(
        &self, event: &str, data: &serde_json::Value,
    ) -> Result<(), AppError> {
        let val = serde_json::to_string(data)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        self.bus.publish_str(event, &val).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn incr(&self, key: &str) -> Result<i64, AppError> {
        self.bus.incr(key).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn exists(&self, key: &str) -> Result<bool, AppError> {
        self.bus.exists(key).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn expire(&self, key: &str, seconds: u64) -> Result<(), AppError> {
        self.bus.expire(key, seconds).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn sadd(&self, key: &str, member: &str) -> Result<(), AppError> {
        self.bus.sadd(key, member).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn srem(&self, key: &str, member: &str) -> Result<(), AppError> {
        self.bus.srem(key, member).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn smembers(&self, key: &str) -> Result<Vec<String>, AppError> {
        self.bus.smembers(key).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn hset(&self, key: &str, field: &str, value: &str) -> Result<(), AppError> {
        self.bus.hset(key, field, value).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn hmset(&self, key: &str, fields: &[(&str, &str)]) -> Result<(), AppError> {
        self.bus.hmset(key, fields).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn hget(&self, key: &str, field: &str) -> Result<Option<String>, AppError> {
        self.bus.hget(key, field).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn hgetall(&self, key: &str) -> Result<std::collections::HashMap<String, String>, AppError> {
        self.bus.hgetall(key).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }
}