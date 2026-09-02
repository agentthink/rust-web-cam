use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use crate::context::{InfraContext, MediaContext};
use crate::domain::{Session, SessionType, SessionState};
use crate::adapter::media_server::StreamInfo;
use crate::domain::traits::CacheStore;
use crate::error::{AppError, Result};
use crate::protocol::event::SignalEvent;

pub struct SessionService {
    infra: InfraContext,
    media: MediaContext,
    shutdown_tx: Arc<RwLock<Option<tokio::sync::watch::Sender<()>>>>,
}

impl SessionService {
    pub fn new(infra: InfraContext, media: MediaContext) -> Self {
        Self {
            infra,
            media,
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(&self, expiration_secs: i64) {
        let (tx, _rx) = tokio::sync::watch::channel(());
        *self.shutdown_tx.write().await = Some(tx);

        let infra = self.infra.clone();
        let media = self.media.clone();
        let mut event_rx = self.infra.subscribe_events();

        tokio::spawn(async move {
            tracing::info!("[SessionService] Started event listener");
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = Self::handle_event(&infra, &media, &event).await {
                            tracing::error!("[SessionService] Event handler error: {}", e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[SessionService] Lagged {} events", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        let repo = self.infra.db.clone();
        let redis = self.infra.redis.clone();
        let cluster = self.media.cluster.clone();
        let exp_secs = expiration_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            tracing::info!("[SessionService] Started cleanup task");
            loop {
                interval.tick().await;
                Self::cleanup_expired_sessions(&repo, &redis, &cluster, exp_secs).await;
            }
        });

        tracing::info!("[SessionService] Started");
    }

    pub async fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.read().await.as_ref() {
            let _ = tx.send(());
        }
    }

    async fn handle_event(
        infra: &InfraContext,
        _media: &MediaContext,
        event: &SignalEvent,
    ) -> anyhow::Result<()> {
        match event {
            SignalEvent::StartPlay { device_id, session_id, .. } => {
                tracing::info!("[SessionService] StartPlay: device={} session={}", device_id, session_id);
                let session = Session::new(SessionType::Play, 0i64);
                infra.db.sessions_cache().insert(session.id, session.clone());
                if let Err(e) = infra.db.create_session(&session).await {
                    tracing::error!("[SessionService] Failed to persist session: {}", e);
                }
            }

            SignalEvent::StopPlay { device_id: _, session_id, .. } => {
                tracing::info!("[SessionService] StopPlay: session={}", session_id);
                if let Some(session) = infra.db.sessions_cache()
                    .iter()
                    .find(|s| s.stream_key() == *session_id)
                    .map(|s| s.clone())
                {
                    infra.db.sessions_cache().remove(&session.id);
                    if let Err(e) = infra.db.delete_session(session.id).await {
                        tracing::error!("[SessionService] Failed to delete session {}: {}", session.id, e);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn create_session(&self, session_type: SessionType, user_id: i64) -> Result<Session> {
        let session = Session::new(session_type, user_id);
        self.infra.db.sessions_cache().insert(session.id, session.clone());
        if let Err(e) = self.infra.db.create_session(&session).await {
            tracing::error!("[SessionService] Failed to persist session: {}", e);
        }
        Ok(session)
    }

    pub async fn activate_session(&self, id: i64, rtsp_url: &str) -> Result<StreamInfo> {
        let session = self.get_session(id)?
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))?;

        let server = self.media.cluster.select_server().await
            .ok_or_else(|| AppError::MediaServerError("No available media server".to_string()))?;

        let device_tag = session.device_tag.as_ref()
            .ok_or_else(|| AppError::NotFound("Session has no device_tag".to_string()))?;
        let device = self.infra.db.get_device_by_device_tag(device_tag)
            .ok_or_else(|| AppError::NotFound(format!("Device with tag {} not found", device_tag)))?;
        let app = device.app.unwrap_or_else(|| "live".to_string());
        
        let stream_key = crate::domain::stream::make_stream_key(
            session.device_tag.as_deref().unwrap_or(""),
            session.channel_tag.as_deref().unwrap_or("")
        );
        let stream_info = server.add_stream_proxy(&app, &stream_key, rtsp_url).await?;

        self.infra.redis.set_stream_info(&stream_key, &stream_info).await?;

        let mut session = session;
        session.set_active(server.name().to_string());
        self.infra.db.sessions_cache().insert(id, session);

        Ok(stream_info)
    }

    pub async fn deactivate_session(&self, id: i64) -> Result<()> {
        let session = self.get_session(id)?
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))?;

        let stream_key = session.stream_key();
        if !stream_key.is_empty() {
            if let Some(server_id) = &session.media_server_tag {
                if let Some(server) = self.media.cluster.get_all_servers().iter().find(|s| s.name() == server_id) {
                    if let Err(e) = server.remove_stream_proxy("live", &stream_key).await {
                        tracing::warn!("[SessionService] Failed to remove stream: {}", e);
                    }
                }
            }
            if let Err(e) = self.infra.redis.delete_stream_info(&stream_key).await {
                tracing::warn!("[SessionService] Failed to delete stream info: {}", e);
            }
        }

        let mut session = session;
        session.terminate();
        self.infra.db.sessions_cache().insert(id, session);
        Ok(())
    }

    pub fn get_session(&self, id: i64) -> Result<Option<Session>> {
        Ok(self.infra.db.sessions_cache().get(&id).map(|s| s.clone()))
    }

    pub fn get_active_count(&self) -> usize {
        self.infra.db.sessions_cache().iter()
            .filter(|s| s.state == SessionState::Active)
            .count()
    }

    pub async fn list_sessions_paginated(&self, limit: usize, offset: usize) -> Vec<Session> {
        self.infra.db.list_sessions_paginated(limit, offset).await
    }

    pub async fn count_sessions(&self) -> usize {
        self.infra.db.count_sessions().await
    }

    async fn cleanup_expired_sessions(
        repo: &Arc<crate::infrastructure::DbRepository>,
        redis: &Arc<crate::infrastructure::RedisStore>,
        cluster: &Arc<crate::infrastructure::cluster::ClusterManager>,
        _expiration_secs: i64,
    ) {
        let mut to_delete = Vec::new();

        for entry in repo.sessions_cache().iter() {
            let session = entry.value();
            if session.state == SessionState::Terminated { continue; }
            if session.is_expired() {
                tracing::info!("[SessionService] Session {} expired", session.id);
                let stream_key = session.stream_key();
                if !stream_key.is_empty() {
                    if let Some(server_id) = &session.media_server_tag {
                        if let Some(server) = cluster.get_all_servers().iter().find(|s| s.name() == server_id) {
                            let _ = server.remove_stream_proxy("live", &stream_key).await;
                        }
                        let _ = redis.delete_stream_info(&stream_key).await;
                    }
                }
                let mut s = session.clone();
                s.mark_terminated();
                repo.sessions_cache().insert(s.id, s);
                to_delete.push(session.id);
            }
        }

        for session_id in to_delete {
            if let Err(e) = repo.delete_session(session_id).await {
                tracing::error!("[SessionService] Failed to delete expired session {}: {}", session_id, e);
            }
        }
    }
}

impl Clone for SessionService {
    fn clone(&self) -> Self {
        Self {
            infra: self.infra.clone(),
            media: self.media.clone(),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }
}