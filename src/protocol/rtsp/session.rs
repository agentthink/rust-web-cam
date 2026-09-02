use crate::protocol::rtsp::sdp::{SdpInfo, SdpTrack};
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

const MAX_RTSP_SESSIONS: usize = 10000;

static RTSP_SESSION_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtspSessionState {
    Init,
    Announced,
    Ready,
    Playing,
    Paused,
}

impl Default for RtspSessionState {
    fn default() -> Self {
        RtspSessionState::Init
    }
}

#[derive(Debug, Clone)]
pub struct RtspSession {
    pub session_id: String,
    pub stream_key: String,
    pub state: RtspSessionState,
    pub cseq: u32,
    pub remote_addr: SocketAddr,
    pub device_sdp: Option<SdpInfo>,
    pub rtp_port: Option<u16>,
    pub media_server_name: Option<String>,
    pub tracks: Vec<SdpTrack>,
    pub created_at: std::time::Instant,
    pub last_activity: std::time::Instant,
}

impl RtspSession {
    pub fn new(stream_key: String, remote_addr: SocketAddr) -> Self {
        let session_id = Uuid::new_v4().to_string();
        Self {
            session_id,
            stream_key: stream_key.clone(),
            state: RtspSessionState::Init,
            cseq: 0,
            remote_addr,
            device_sdp: None,
            rtp_port: None,
            media_server_name: None,
            tracks: Vec::new(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    pub fn is_timeout(&self, timeout_secs: u64) -> bool {
        self.last_activity.elapsed().as_secs() > timeout_secs
    }
}

static RTSP_SESSIONS: once_cell::sync::Lazy<
    DashMap<String, Arc<tokio::sync::RwLock<RtspSession>>>,
> = once_cell::sync::Lazy::new(|| DashMap::new());

pub fn create_session(
    key: String,
    addr: SocketAddr,
) -> Option<Arc<tokio::sync::RwLock<RtspSession>>> {
    if RTSP_SESSIONS.len() >= MAX_RTSP_SESSIONS {
        tracing::warn!(
            "[RTSP] Max sessions {} reached, rejecting new session",
            MAX_RTSP_SESSIONS
        );
        return None;
    }
    let session = RtspSession::new(key, addr);
    let session_id = session.session_id.clone();
    let arc = Arc::new(tokio::sync::RwLock::new(session));
    RTSP_SESSIONS.insert(session_id, arc.clone());
    let count = RTSP_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    tracing::debug!("[RTSP] Session created (total: {})", count + 1);
    Some(arc)
}

pub fn get_session(id: &str) -> Option<Arc<tokio::sync::RwLock<RtspSession>>> {
    RTSP_SESSIONS.get(id).map(|r| r.value().clone())
}

pub fn remove_session(id: &str) {
    if RTSP_SESSIONS.remove(id).is_some() {
        tracing::debug!("[RTSP] Session {} removed", id);
    }
}

pub fn session_count() -> usize {
    RTSP_SESSIONS.len()
}

pub fn get_session_by_stream_key(
    stream_key: &str,
) -> Option<Arc<tokio::sync::RwLock<RtspSession>>> {
    RTSP_SESSIONS
        .iter()
        .find(|r| r.value().blocking_read().stream_key == stream_key)
        .map(|r| r.value().clone())
}

pub fn cleanup_timed_out_sessions(timeout_secs: u64) -> usize {
    let ids: Vec<String> = RTSP_SESSIONS
        .iter()
        .filter(|r| r.value().blocking_read().is_timeout(timeout_secs))
        .map(|r| r.key().clone())
        .collect();
    for id in &ids {
        RTSP_SESSIONS.remove(id);
    }
    if !ids.is_empty() {
        tracing::info!("[RTSP] Cleaned up {} timed-out sessions", ids.len());
    }
    ids.len()
}

pub async fn cleanup_expired(cluster: Arc<crate::infrastructure::cluster::ClusterManager>) -> usize {
    let ids: Vec<String> = RTSP_SESSIONS
        .iter()
        .filter(|r| r.value().blocking_read().is_timeout(60))
        .map(|r| r.key().clone())
        .collect();

    let mut removed = 0;
    for id in ids {
        let Some(sess_guard) = RTSP_SESSIONS.get(&id) else {
            continue;
        };
        let sess_guard = sess_guard; // suppress warning
        let session = sess_guard.blocking_read();

        let tag = match &session.media_server_name {
            Some(t) => t.clone(),
            None => {
                drop(session);
                RTSP_SESSIONS.remove(&id);
                continue;
            }
        };

        let stream_id = session.stream_key.clone();
        drop(session);

        if stream_id.is_empty() {
            RTSP_SESSIONS.remove(&id);
            continue;
        }

        if let Some(adapter) = cluster.get_server(&tag) {
            if let Err(e) = adapter.close_rtp_server(&stream_id).await {
                tracing::warn!("[RTSP] Cleanup: failed to close RTP server {} on {}: {}", stream_id, tag, e);
            } else {
                tracing::info!("[RTSP] Cleanup: closed RTP server {} on {}", stream_id, tag);
            }
        }
        RTSP_SESSIONS.remove(&id);
        removed += 1;
    }
    removed
}
