use std::sync::Arc;
use std::net::SocketAddr;
use async_trait::async_trait;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::AsyncWriteExt;

use crate::protocol::adapter::SignalAdapter;
use crate::protocol::event::{SignalEvent, ProtocolType, TransportType, PtzCommand};
use crate::protocol::traits::ProtocolDeps;
use crate::error::{Result, AppError};  // ← 导入 crate::error::Result

/// WebSocket 帧操作码
#[derive(Debug, Clone, Copy, PartialEq)]
enum WsOpcode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl WsOpcode {
    fn from(u: u8) -> Option<Self> {
        match u & 0xF {
            0x0 => Some(Self::Continuation),
            0x1 => Some(Self::Text),
            0x2 => Some(Self::Binary),
            0x8 => Some(Self::Close),
            0x9 => Some(Self::Ping),
            0xA => Some(Self::Pong),
            _ => None,
        }
    }
}

/// WebSocket 帧
struct WsFrame;

impl WsFrame {
    fn parse(buffer: &[u8]) -> Option<(WsFrameData, usize)> {
        if buffer.len() < 2 { return None; }

        let byte0 = buffer[0];
        let byte1 = buffer[1];
        let fin = (byte0 & 0x80) != 0;
        let opcode = WsOpcode::from(byte0 & 0xF)?;
        let masked = (byte1 & 0x80) != 0;
        let mut payload_len = (byte1 & 0x7F) as usize;
        let mut header_len = 2;

        if payload_len == 126 {
            if buffer.len() < 4 { return None; }
            payload_len = ((buffer[2] as usize) << 8) | (buffer[3] as usize);
            header_len = 4;
        } else if payload_len == 127 {
            if buffer.len() < 10 { return None; }
            payload_len = 0;
            for i in 0..8 { payload_len = (payload_len << 8) | (buffer[2 + i] as usize); }
            header_len = 10;
        }

        let mask_len = if masked { 4 } else { 0 };
        let total_len = header_len + mask_len + payload_len;
        if buffer.len() < total_len { return None; }

        let payload_start = header_len + mask_len;
        let mut payload = buffer[payload_start..payload_start + payload_len].to_vec();

        if masked {
            let mask = &buffer[header_len..header_len + 4];
            for (i, byte) in payload.iter_mut().enumerate() { *byte ^= mask[i % 4]; }
        }

        Some((WsFrameData { opcode, fin, payload }, total_len))
    }

    fn build_text_frame(data: &str) -> Vec<u8> {
        let payload = data.as_bytes();
        let payload_len = payload.len();
        let mut frame = Vec::new();
        frame.push(0x81);
        if payload_len < 126 {
            frame.push(payload_len as u8);
        } else if payload_len <= 65535 {
            frame.push(126);
            frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
        } else {
            frame.push(127);
            frame.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }
        frame.extend_from_slice(payload);
        frame
    }

    fn build_close_frame() -> Vec<u8> { vec![0x88, 0x00] }

    fn build_pong_frame(data: &[u8]) -> Vec<u8> {
        let payload_len = data.len();
        let mut frame = Vec::new();
        frame.push(0x8A);
        if payload_len < 126 {
            frame.push(payload_len as u8);
        } else if payload_len <= 65535 {
            frame.push(126);
            frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
        } else {
            frame.push(127);
            frame.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }
        frame.extend_from_slice(data);
        frame
    }
}

struct WsFrameData {
    opcode: WsOpcode,
    fin: bool,
    payload: Vec<u8>,
}

/// WebSocket 协议适配器
pub struct WebSocketAdapter {
    remote_addr: Option<SocketAddr>,
    device_id: Option<String>,
    authenticated: bool,
    recv_buffer: Vec<u8>,
    cont_payload: Vec<u8>,
    write: Option<Arc<tokio::sync::RwLock<OwnedWriteHalf>>>,
    deps: ProtocolDeps,
}

impl WebSocketAdapter {
    pub fn new(deps: ProtocolDeps) -> Self {
        Self {
            remote_addr: None,
            device_id: None,
            authenticated: false,
            recv_buffer: Vec::new(),
            cont_payload: Vec::new(),
            write: None,
            deps,
        }
    }

    fn parse_json_message(&mut self, data: &[u8]) -> Result<Vec<SignalEvent>> {
        let data_str = String::from_utf8_lossy(data);
        let mut events = Vec::new();

        let json: serde_json::Value = match serde_json::from_str(&data_str) {
            Ok(j) => j,
            Err(e) => {
                tracing::debug!("[WebSocket] Invalid JSON: {}", e);
                return Ok(events);
            }
        };

        let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match msg_type {
            "login" | "auth" => {
                let device_id = json.get("device_id").and_then(|v| v.as_str()).unwrap_or("ws_device");
                let token = json.get("token").and_then(|v| v.as_str());

                let authenticated = if let Some(t) = token {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(self.deps.stream_manager.validate_token(t)).is_some()
                } else {
                    true
                };

                if authenticated {
                    self.device_id = Some(device_id.to_string());
                    self.authenticated = true;
                    events.push(SignalEvent::DeviceRegister {
                        device_id: 0,
                        device_tag: None,
                        name: json.get("name").and_then(|v| v.as_str()).unwrap_or("WebSocket Device").to_string(),
                        stream_key: None,
                        manufacturer: json.get("manufacturer").and_then(|v| v.as_str()).map(String::from),
                        model: json.get("model").and_then(|v| v.as_str()).map(String::from),
                        protocol: ProtocolType::WebRtc,
                    });

                    let response = serde_json::json!({
                        "type": "login_response",
                        "status": "ok",
                        "device_id": device_id,
                    });
                    if let Some(ref write_arc) = self.write {
                        let frame = WsFrame::build_text_frame(&response.to_string());
                        let handle = tokio::runtime::Handle::current();
                        let _ = handle.block_on(async {
                            write_arc.write().await.write_all(&frame).await
                        });
                    }
                }
            }

            "start_play" => {
                if let Some(device_id_str) = json.get("device_id").and_then(|v| v.as_str()) {
                    let device_id = device_id_str.parse().unwrap_or(0);
                    events.push(SignalEvent::StartPlay {
                        device_id,
                        device_tag: None,
                        session_id: json.get("session_id").and_then(|v| v.as_str()).unwrap_or("ws_session").to_string(),
                        channel_id: json.get("channel").and_then(|v| v.as_str()).map(String::from),
                        transport: TransportType::WebSocket,
                        media_server_name: None,
                    });
                }
            }

            "stop_play" => {
                if let Some(device_id_str) = json.get("device_id").and_then(|v| v.as_str()) {
                    let device_id = device_id_str.parse().unwrap_or(0);
                    events.push(SignalEvent::StopPlay {
                        device_id,
                        device_tag: None,
                        session_id: json.get("session_id").and_then(|v| v.as_str()).unwrap_or("ws_session").to_string(),
                    });
                }
            }

            "ptz" => {
                if let Some(device_id) = json.get("device_id").and_then(|v| v.as_str()) {
                    let cmd_str = json.get("command").and_then(|v| v.as_str()).unwrap_or("stop");
                    let command = match cmd_str {
                        "up" => PtzCommand::Up,
                        "down" => PtzCommand::Down,
                        "left" => PtzCommand::Left,
                        "right" => PtzCommand::Right,
                        "zoom_in" => PtzCommand::ZoomIn,
                        "zoom_out" => PtzCommand::ZoomOut,
                        "stop" => PtzCommand::Stop,
                        _ => PtzCommand::Stop,
                    };
                    let speed = json.get("speed").and_then(|v| v.as_u64()).map(|v| v as u8);
                    events.push(SignalEvent::PtzControl { device_id: device_id.to_string(), command, speed });
                }
            }

            "alarm" => {
                if let Some(device_id) = json.get("device_id").and_then(|v| v.as_str()) {
                    events.push(SignalEvent::Alarm {
                        device_id: device_id.to_string(),
                        alarm_type: json.get("alarm_type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                        message: json.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }

            "keepalive" | "ping" => {
                if let Some(device_id) = &self.device_id {
                    let device_id_num = device_id.parse().unwrap_or(0);
                    events.push(SignalEvent::DeviceKeepalive {
                        device_id: device_id_num,
                        device_tag: None,
                        timestamp: chrono::Utc::now(),
                    });
                }
            }

            _ => {}
        }

        Ok(events)
    }
}

#[async_trait]
impl SignalAdapter for WebSocketAdapter {
    async fn parse(&mut self, data: &[u8]) -> Result<Vec<SignalEvent>> {
        self.recv_buffer.extend_from_slice(data);
        let mut events = Vec::new();

        loop {
            match WsFrame::parse(&self.recv_buffer) {
                Some((frame, consumed)) => {
                    self.recv_buffer = self.recv_buffer[consumed..].to_vec();

                    match frame.opcode {
                        WsOpcode::Continuation => {
                            self.cont_payload.extend_from_slice(&frame.payload);
                            if frame.fin {
                                let payload = std::mem::take(&mut self.cont_payload);
                                match self.parse_json_message(&payload) {
                                    Ok(msgs) => events.extend(msgs),
                                    Err(e) => tracing::error!("[WebSocket] Parse error: {}", e),
                                }
                            }
                        }
                        WsOpcode::Text | WsOpcode::Binary => {
                            if frame.fin {
                                match self.parse_json_message(&frame.payload) {
                                    Ok(msgs) => events.extend(msgs),
                                    Err(e) => tracing::error!("[WebSocket] Parse error: {}", e),
                                }
                            } else {
                                self.cont_payload = frame.payload;
                            }
                        }
                        WsOpcode::Ping => {
                            let pong = WsFrame::build_pong_frame(&frame.payload);
                            if let Some(ref write_arc) = self.write {
                                let _ = write_arc.write().await.write_all(&pong).await;
                            }
                        }
                        WsOpcode::Pong => {}
                        WsOpcode::Close => {
                            tracing::info!("[WebSocket] Close frame received");
                            if let Some(ref write_arc) = self.write {
                                let close = WsFrame::build_close_frame();
                                let _ = write_arc.write().await.write_all(&close).await;
                            }
                            break;
                        }
                    }
                }
                None => break,
            }
        }

        Ok(events)
    }

    async fn on_connected(&mut self, addr: SocketAddr) -> Result<()> {
        self.remote_addr = Some(addr);
        tracing::info!("[WebSocket] Connection from {}", addr);
        Ok(())
    }

    async fn on_disconnected(&mut self) -> Result<()> {
        if let Some(ref device_id) = self.device_id {
            tracing::info!("[WebSocket] Device {} disconnected", device_id);
            let _ = self.deps.device_lookup.set_offline(device_id, Some("Connection closed")).await;
        }
        Ok(())
    }

    // ✅ 修复：返回类型改为 crate::error::Result
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref write_arc) = self.write {
            let mut write = write_arc.write().await;
            write.write_all(data).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        Ok(())
    }

    fn protocol_type(&self) -> ProtocolType { ProtocolType::WebRtc }
    fn name(&self) -> &'static str { "WebSocket" }
    fn keepalive(&self) -> bool { true }
    fn idle_timeout(&self) -> Option<u64> { Some(300) }

    fn set_tcp_write(&mut self, write: OwnedWriteHalf) {
        self.write = Some(Arc::new(tokio::sync::RwLock::new(write)));
    }

    async fn start(&mut self, _device_tag: &str) -> Result<()> { Ok(()) }
    async fn ptz_control(&mut self, _channel_id: &str, _command: &crate::protocol::event::PtzCommand, _speed: Option<u8>) -> Result<()> { Ok(()) }
}