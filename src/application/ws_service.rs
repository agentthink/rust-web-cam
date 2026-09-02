use std::sync::Arc;
use std::time::Duration;
use crate::application::ws_broadcaster::WsBroadcaster;
use crate::application::{DeviceService, SessionService};
use crate::infrastructure::cluster::ClusterManager;

pub struct WsService {
    broadcaster: Arc<WsBroadcaster>,
    device_service: Arc<DeviceService>,
    session_service: Arc<SessionService>,
    cluster_manager: Arc<ClusterManager>,
}

impl WsService {
    pub fn new(
        broadcaster: Arc<WsBroadcaster>,
        device_service: Arc<DeviceService>,
        session_service: Arc<SessionService>,
        cluster_manager: Arc<ClusterManager>,
    ) -> Self {
        Self { broadcaster, device_service, session_service, cluster_manager }
    }

    pub fn start(self: Arc<Self>) {
        let broadcaster = self.broadcaster.clone();
        let device_service = self.device_service.clone();
        let session_service = self.session_service.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let stats = serde_json::json!({
                    "devices": {
                        "total": device_service.get_stats()["total"],
                        "online": device_service.get_stats()["online"],
                    },
                    "sessions": { "active": session_service.get_active_count() },
                });
                broadcaster.broadcast("stats_update", stats);
            }
        });

        tracing::info!("[WsService] Started periodic stats push");
    }
}