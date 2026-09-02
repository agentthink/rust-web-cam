use chrono::Utc;
use serde_json::json;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    pub msg_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct WsBroadcaster {
    pub sender: broadcast::Sender<WsMessage>,
}

impl WsBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    pub fn sender(&self) -> broadcast::Sender<WsMessage> {
        self.sender.clone()
    }

    pub fn broadcast(&self, msg_type: &str, data: serde_json::Value) {
        let msg = WsMessage {
            msg_type: msg_type.to_string(),
            data,
            timestamp: Utc::now().to_rfc3339(),
        };
        let _ = self.sender.send(msg);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsMessage> {
        self.sender.subscribe()
    }

    pub fn device_online(&self, device_id: i64) {
        self.broadcast("device_online", json!({ "device_id": device_id }));
    }

    pub fn device_offline(&self, device_id: i64, reason: Option<&str>) {
        self.broadcast(
            "device_offline",
            json!({ "device_id": device_id, "reason": reason }),
        );
    }

    pub fn alarm(&self, device_id: i64, alarm_type: &str, message: &str) {
        self.broadcast(
            "alarm",
            json!({ "device_id": device_id, "alarm_type": alarm_type, "message": message }),
        );
    }

    pub fn ptz_result(
        &self,
        device_id: i64,
        call_id: &str,
        command: &str,
        status: &str,
        sip_code: Option<u16>,
        message: Option<&str>,
    ) {
        self.broadcast(
            "ptz_result",
            json!({
                "device_id": device_id, "call_id": call_id, "command": command,
                "status": status, "sip_code": sip_code, "message": message,
            }),
        );
    }

    pub fn server_online(&self, server_name: &str) {
        self.broadcast("server_online", json!({ "server_name": server_name }));
    }

    pub fn server_offline(&self, server_name: &str) {
        self.broadcast("server_offline", json!({ "server_name": server_name }));
    }
}

impl Default for WsBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
