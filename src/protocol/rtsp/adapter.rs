use async_trait::async_trait;
use std::net::SocketAddr;
use crate::protocol::event::{SignalEvent, ProtocolType, PtzCommand};
use crate::protocol::adapter::SignalAdapter;
use crate::error::Result;

pub struct RtspAdapter {
    remote_addr: Option<SocketAddr>,
    device_id: Option<String>,
    cseq: u32,
    session: Option<String>,
    recv_buffer: Vec<u8>,
}

impl RtspAdapter {
    pub fn new() -> Self {
        Self {
            remote_addr: None,
            device_id: None,
            cseq: 0,
            session: None,
            recv_buffer: Vec::new(),
        }
    }

    fn next_cseq(&mut self) -> u32 {
        self.cseq += 1;
        self.cseq
    }

    fn parse_headers(data: &[u8]) -> Option<(std::collections::HashMap<String, String>, usize)> {
        let data_str = String::from_utf8_lossy(data);
        let header_end = data_str.find("\r\n\r\n")?;
        let header_str = &data_str[..header_end];
        let mut headers = std::collections::HashMap::new();
        for line in header_str.lines() {
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }
        Some((headers, header_end + 4))
    }

    fn extract_message(buffer: &[u8]) -> Option<(&[u8], &[u8])> {
        let (headers, body_start) = Self::parse_headers(buffer)?;
        let content_length = headers
            .get("content-length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let total_len = body_start + content_length;
        if buffer.len() < total_len { return None; }
        Some((&buffer[..total_len], &buffer[total_len..]))
    }

    pub fn parse_message(&mut self, data: &[u8]) -> Result<Vec<SignalEvent>> {
        let data_str = String::from_utf8_lossy(data);
        let mut events = Vec::new();
        let lines: Vec<&str> = data_str.lines().collect();

        for line in &lines {
            let line = line.trim();
            if line.starts_with("OPTIONS ") || line.starts_with("DESCRIBE ") ||
                line.starts_with("SETUP ") || line.starts_with("PLAY ") ||
                line.starts_with("TEARDOWN ") {
                events.push(SignalEvent::StartPlay {
                    device_id: 0,
                    device_tag: None,
                    session_id: self.session.clone().unwrap_or_else(|| format!("rtsp_{}", chrono::Utc::now().timestamp())),
                    channel_id: Some("0".to_string()),
                    transport: crate::protocol::event::TransportType::TCP,
                    media_server_name: None,
                });
            }
            if line.starts_with("CSeq:") {
                if let Ok(seq) = line.trim_start_matches("CSeq:").trim().parse::<u32>() {
                    self.cseq = seq;
                }
            }
        }

        Ok(events)
    }

    pub fn build_options_response(&mut self) -> String {
        let cseq = self.next_cseq();
        format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nPublic: DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE\r\n\r\n", cseq)
    }

    pub fn build_describe_response(&mut self, sdp: &str) -> String {
        let cseq = self.next_cseq();
        format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{}", cseq, sdp.len(), sdp)
    }
}

impl Default for RtspAdapter {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl SignalAdapter for RtspAdapter {
    async fn parse(&mut self, data: &[u8]) -> Result<Vec<SignalEvent>> {
        self.recv_buffer.extend_from_slice(data);
        let mut events = Vec::new();

        loop {
            let buffer = self.recv_buffer.clone();
            let (msg, remainder) = match Self::extract_message(&buffer) {
                Some((msg, remainder)) => (msg.to_vec(), remainder.to_vec()),
                None => break,
            };
            self.recv_buffer = remainder;
            let msg_events = self.parse_message(&msg)?;
            events.extend(msg_events);
        }

        Ok(events)
    }

    async fn on_connected(&mut self, addr: SocketAddr) -> Result<()> {
        self.remote_addr = Some(addr);
        tracing::info!("[RTSP] Connection from {}", addr);
        Ok(())
    }

    async fn on_disconnected(&mut self) -> Result<()> {
        tracing::info!("[RTSP] Device disconnected");
        Ok(())
    }

    async fn send(&mut self, _data: &[u8]) -> Result<()> { Ok(()) }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::Rtsp }
    fn name(&self) -> &'static str { "RTSP" }
    fn keepalive(&self) -> bool { true }
    fn idle_timeout(&self) -> Option<u64> { Some(60) }
    async fn start(&mut self, _device_tag: &str) -> Result<()> { Ok(()) }
    async fn ptz_control(&mut self, _channel_id: &str, _command: &crate::protocol::event::PtzCommand, _speed: Option<u8>) -> Result<()> { Ok(()) }
}