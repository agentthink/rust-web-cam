use std::sync::Arc;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::AsyncWriteExt;
use async_trait::async_trait;

use crate::adapter::media_server::RtpTransport;
use crate::protocol::adapter::SignalAdapter;
use crate::protocol::event::{SignalEvent, ProtocolType, TransportType, PtzCommand};
use crate::protocol::traits::ProtocolDeps;
use crate::protocol::rtsp::auth::{
    RtspAuthContext, authenticate, generate_nonce, build_www_authenticate,
};
use crate::protocol::rtsp::response::RtspResponse;
use crate::protocol::rtsp::sdp::SdpParser;
use crate::protocol::rtsp::session::{
    create_session, get_session, remove_session, get_session_by_stream_key,
    RtspSessionState,
};
use crate::error::{AppError, Result};

const MAX_INTERLEAVED_BUFFER: usize = 1024 * 1024;

pub struct RtspServerAdapter {
    recv_buffer: Vec<u8>,
    session: Option<String>,
    cseq: u32,
    remote_addr: Option<SocketAddr>,
    write: Option<Arc<tokio::sync::RwLock<OwnedWriteHalf>>>,
    interleaved_buffer: Vec<u8>,
    deps: ProtocolDeps,
}

impl RtspServerAdapter {
    pub fn new(deps: ProtocolDeps) -> Self {
        Self {
            recv_buffer: Vec::new(),
            session: None,
            cseq: 0,
            remote_addr: None,
            write: None,
            interleaved_buffer: Vec::new(),
            deps,
        }
    }

    // ═══════════════════════════════════════════════════════
    // RTSP 消息解析
    // ═══════════════════════════════════════════════════════

    fn parse_request_line(line: &str) -> Option<(String, String, String)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            Some((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
        } else {
            None
        }
    }

    fn parse_headers(lines: &[&str]) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }
        headers
    }

    pub fn extract_stream_key(url: &str) -> String {
        let url = url.trim_start_matches("rtsp://");
        let parts: Vec<&str> = url.splitn(2, '/').collect();
        if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            parts[0].to_string()
        }
    }

    fn parse_client_ports(transport: &str) -> Option<(u16, u16)> {
        for part in transport.split(';') {
            let part = part.trim();
            if part.starts_with("client_port=") {
                let val = &part[12..];
                let ports: Vec<&str> = val.split('-').collect();
                if ports.len() == 2 {
                    return Some((ports[0].parse().ok()?, ports[1].parse().ok()?));
                }
            }
        }
        None
    }

    fn parse_interleaved_channels(transport: &str) -> Option<(u8, u8)> {
        for part in transport.split(';') {
            let part = part.trim();
            if part.starts_with("interleaved=") {
                let val = &part[12..];
                let channels: Vec<&str> = val.split('-').collect();
                if channels.len() == 2 {
                    return Some((channels[0].parse().ok()?, channels[1].parse().ok()?));
                }
            }
        }
        None
    }

    fn extract_rtsp_message(buffer: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        let data_str = String::from_utf8_lossy(buffer);
        let header_end = data_str.find("\r\n\r\n")?;

        let content_length = data_str[..header_end]
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split_once(':')?.1.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let body_start = header_end + 4;
        let total_len = body_start + content_length;
        if buffer.len() < total_len {
            return None;
        }

        Some((buffer[..total_len].to_vec(), buffer[total_len..].to_vec()))
    }

    // ═══════════════════════════════════════════════════════
    // 默认 SDP Track
    // ═══════════════════════════════════════════════════════

    fn default_h264_track() -> Vec<crate::protocol::rtsp::sdp::SdpTrack> {
        vec![crate::protocol::rtsp::sdp::SdpTrack {
            media: "video".to_string(),
            payload_type: 96,
            codec: "H264".to_string(),
            clock_rate: 90000,
            fmtp: Some("packetization-mode=1".to_string()),
            control: None,
        }]
    }

    // ═══════════════════════════════════════════════════════
    // 发送方法
    // ═══════════════════════════════════════════════════════

    async fn send_raw(&self, data: &[u8]) -> Result<()> {
        if let Some(ref write_arc) = self.write {
            let mut write = write_arc.write().await;
            write.write_all(data).await
                .map_err(|e| AppError::Internal(format!("TCP write error: {}", e)))?;
            let _ = write.flush().await;
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // RTP 端口分配
    // ═══════════════════════════════════════════════════════

    async fn alloc_rtp_port(&self, stream_key: &str, preferred_server: Option<&str>) -> Result<(u16, String, String)> {
        let preferred = if let Some(name) = preferred_server {
            Some(name.to_string())
        } else {
            self.deps.device_lookup.find_by_stream_key(stream_key).await
                .and_then(|d| d.media_server_tag.clone())
        };

        if let Some(ref name) = preferred {
            if let Some(adapter) = self.deps.cluster.get_server(name) {
                if adapter.is_online().await {
                    let server_tag = adapter.tag().to_string();
                    match adapter.open_rtp_server(stream_key, 0, RtpTransport::Udp).await {
                        Ok(result) => {
                            tracing::info!(
                                "[RTSP] Allocated RTP (preferred): stream={}, port={}, ip={}, server={}",
                                stream_key, result.0, result.1, server_tag
                            );
                            return Ok((result.0, result.1, server_tag));
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[RTSP] Preferred server {} failed for stream={}: {}, falling back",
                                name, stream_key, e
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        "[RTSP] Preferred server {} is offline for stream={}, falling back",
                        name, stream_key
                    );
                }
            } else {
                tracing::warn!(
                    "[RTSP] Preferred server {} not found for stream={}, falling back",
                    name, stream_key
                );
            }
        }

        if let Some(adapter) = self.deps.cluster.select_server().await {
            let server_tag = adapter.tag().to_string();
            match adapter.open_rtp_server(stream_key, 0, RtpTransport::Udp).await {
                Ok(result) => {
                    tracing::info!(
                        "[RTSP] Allocated RTP (fallback): stream={}, port={}, ip={}, server={}",
                        stream_key, result.0, result.1, server_tag
                    );
                    return Ok((result.0, result.1, server_tag));
                }
                Err(e) => {
                    tracing::warn!("[RTSP] Failed to alloc RTP via cluster: {}", e);
                }
            }
        }

        Err(AppError::MediaServerError("No media server available for RTP allocation".to_string()))
    }

    async fn close_rtp_port(&self, stream_key: &str, server_tag: &str) {
        if let Some(adapter) = self.deps.cluster.get_server(server_tag) {
            if let Err(e) = adapter.close_rtp_server(stream_key).await {
                tracing::warn!("[RTSP] Failed to close old RTP server: {}", e);
            } else {
                tracing::info!("[RTSP] Closed old RTP server: stream={}, server={}", stream_key, server_tag);
            }
        }
    }

    // ═══════════════════════════════════════════════════════
    // 认证检查
    // ═══════════════════════════════════════════════════════

    fn check_auth(
        &self,
        cseq: u32,
        authorization: Option<&str>,
        method: &str,
        uri: &str,
        stream_key: &str,
    ) -> Option<String> {
        let rtsp_auth = &self.deps.config.rtsp_auth;

        if !rtsp_auth.enabled {
            return None;
        }

        let ctx = RtspAuthContext::new(&rtsp_auth.realm)
            .with_defaults(
                rtsp_auth.default_username.clone(),
                rtsp_auth.default_password.clone(),
            )
            .with_enabled(true);

        let (username, password) = match (
            &rtsp_auth.default_username,
            &rtsp_auth.default_password,
        ) {
            (Some(u), Some(p)) => (u.clone(), p.clone()),
            _ => return None,
        };

        let (nonce, _) = generate_nonce();
        let header = build_www_authenticate(&ctx.realm, &nonce);

        if authorization.is_none() {
            tracing::debug!(
                "[RTSP] Auth required for stream={}, method={}",
                stream_key,
                method
            );
            return Some(RtspResponse::unauthorized(cseq, &header));
        }

        match authenticate(&ctx, authorization, method, uri, &username, &password) {
            Ok(()) => None,
            Err(_) => {
                tracing::warn!(
                    "[RTSP] Auth failed for stream={}, method={}",
                    stream_key,
                    method
                );
                Some(RtspResponse::unauthorized(cseq, &header))
            }
        }
    }

    // ═══════════════════════════════════════════════════════
    // Interleaved RTP 数据处理
    // ═══════════════════════════════════════════════════════

    async fn handle_interleaved_data(&mut self, data: &[u8]) -> (Vec<SignalEvent>, Option<String>) {
        if self.interleaved_buffer.len() + data.len() > MAX_INTERLEAVED_BUFFER {
            tracing::error!(
                "[RTSP] Interleaved buffer overflow ({} + {} > {}), clearing",
                self.interleaved_buffer.len(),
                data.len(),
                MAX_INTERLEAVED_BUFFER
            );
            self.interleaved_buffer.clear();
            return (vec![], None);
        }
        self.interleaved_buffer.extend_from_slice(data);
        let mut consumed = 0;
        let stream_key = self.get_session_stream_key().await;

        loop {
            if consumed >= self.interleaved_buffer.len() {
                self.interleaved_buffer.clear();
                break;
            }

            match self
                .deps
                .rtp_tunnel
                .handle_interleaved(&stream_key, &self.interleaved_buffer[consumed..])
                .await
            {
                Ok(Some(frame_len)) => {
                    consumed += frame_len;
                }
                Ok(None) => {
                    if consumed > 0 {
                        self.interleaved_buffer.drain(..consumed);
                    }
                    break;
                }
                Err(e) => {
                    tracing::error!("[RTSP] RTP tunnel error: {}", e);
                    self.interleaved_buffer.clear();
                    break;
                }
            }
        }

        (vec![], None)
    }

    async fn get_session_stream_key(&self) -> String {
        if let Some(ref sess_id) = self.session {
            if let Some(sess) = get_session(sess_id) {
                let s = sess.read().await;
                return s.stream_key.clone();
            }
        }
        "unknown".to_string()
    }

    // ═══════════════════════════════════════════════════════
    // RTSP 请求处理
    // ═══════════════════════════════════════════════════════

    async fn handle_request(
        &mut self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (Vec<SignalEvent>, Option<String>) {
        let mut events = Vec::new();
        let cseq = match headers.get("cseq").and_then(|v| v.parse().ok()) {
            Some(c) => c,
            None => {
                tracing::warn!("[RTSP] Missing CSeq header");
                return (events, Some(RtspResponse::error(0, 400, "Bad Request")));
            }
        };
        self.cseq = cseq;

        let session_header = headers.get("session").cloned();

        match method {
            "OPTIONS" => (events, Some(RtspResponse::options(cseq))),

            "ANNOUNCE" => {
                let stream_key = Self::extract_stream_key(url);
                let authorization = headers.get("authorization").map(|s| s.as_str());

                if let Some(resp) =
                    self.check_auth(cseq, authorization, "ANNOUNCE", url, &stream_key)
                {
                    return (events, Some(resp));
                }

                let sdp_info = SdpParser::parse(body).ok();
                let tracks = sdp_info
                    .as_ref()
                    .map(|s| s.tracks.clone())
                    .unwrap_or_default();
                let device_id = sdp_info
                    .as_ref()
                    .and_then(|s| s.origin.as_ref())
                    .cloned()
                    .unwrap_or_else(|| stream_key.clone());

                let preferred_name = {
                    if let Some(s) = get_session_by_stream_key(&stream_key) {
                        s.read().await.media_server_name.clone()
                    } else {
                        None
                    }
                };
                let preferred: Option<&str> = preferred_name.as_deref();

                let (rtp_port, server_ip, server_name) = match self.alloc_rtp_port(&stream_key, preferred).await {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::error!("[RTSP] ANNOUNCE RTP alloc failed: {}", e);
                        return (
                            events,
                            Some(RtspResponse::error(cseq, 500, "RTP allocation failed")),
                        );
                    }
                };

                let addr = self
                    .remote_addr
                    .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());
                let Some(sess) = create_session(stream_key.clone(), addr) else {
                    tracing::error!("[RTSP] ANNOUNCE: failed to create session (max reached)");
                    return (
                        events,
                        Some(RtspResponse::error(cseq, 503, "Service Unavailable")),
                    );
                };
                {
                    let mut s = sess.write().await;
                    s.device_sdp = sdp_info;
                    s.stream_key = stream_key.clone();
                    s.rtp_port = Some(rtp_port);
                    s.media_server_name = Some(server_name);
                    s.tracks = tracks;
                    s.state = RtspSessionState::Announced;
                }
                self.session = Some(sess.read().await.session_id.clone());

                let transport = format!(
                    "RTP/AVP;unicast;server_port={};source={}",
                    rtp_port, server_ip
                );
                tracing::info!(
                    "[RTSP] ANNOUNCE: stream={}, device={}, rtp={}:{}",
                    stream_key,
                    device_id,
                    server_ip,
                    rtp_port
                );

                events.push(SignalEvent::DeviceRegister {
                    device_id: 0,
                    device_tag: None,
                    name: stream_key.clone(),
                    stream_key: Some(stream_key),
                    manufacturer: None,
                    model: None,
                    protocol: ProtocolType::Rtsp,
                });

                (
                    events,
                    Some(RtspResponse::announce_with_transport(cseq, &transport)),
                )
            }

            "DESCRIBE" => {
                let stream_key = Self::extract_stream_key(url);
                let authorization = headers.get("authorization").map(|s| s.as_str());

                if let Some(resp) =
                    self.check_auth(cseq, authorization, "DESCRIBE", url, &stream_key)
                {
                    return (events, Some(resp));
                }

                let sdp_tracks = if let Some(sess) = get_session_by_stream_key(&stream_key) {
                    let s = sess.read().await;
                    s.device_sdp
                        .as_ref()
                        .map(|d| d.tracks.clone())
                        .unwrap_or_else(|| Self::default_h264_track())
                } else {
                    Self::default_h264_track()
                };

                let sdp = SdpParser::build_sdp(
                    &stream_key,
                    &self.deps.config.server.host,
                    &sdp_tracks,
                );
                tracing::info!("[RTSP] DESCRIBE: stream={}", stream_key);
                (events, Some(RtspResponse::describe(cseq, &sdp)))
            }

            "SETUP" => {
                let stream_key = Self::extract_stream_key(url);
                let sess_id = session_header
                    .as_deref()
                    .unwrap_or("")
                    .trim_start_matches(';');

                let transport_in = headers.get("transport");
                let wants_interleaved = transport_in
                    .map(|t| t.contains("interleaved="))
                    .unwrap_or(false);

                let sess = if sess_id.is_empty() {
                    let addr = self
                        .remote_addr
                        .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());
                    let Some(s) = create_session(stream_key.clone(), addr) else {
                        tracing::error!("[RTSP] SETUP: failed to create session (max reached)");
                        return (
                            events,
                            Some(RtspResponse::error(cseq, 503, "Service Unavailable")),
                        );
                    };
                    self.session = Some(s.read().await.session_id.clone());
                    s
                } else if let Some(s) = get_session(sess_id) {
                    self.session = Some(sess_id.to_string());
                    s
                } else {
                    return (
                        events,
                        Some(RtspResponse::error(cseq, 454, "Session Not Found")),
                    );
                };

                let (client_rtp, client_rtcp) = transport_in
                    .and_then(|t| Self::parse_client_ports(t))
                    .unwrap_or((0, 0));

                // 关闭旧 RTP 端口（防止端口泄漏）
                if let Some(ref old_server) = sess.read().await.media_server_name {
                    let old_stream_key = sess.read().await.stream_key.clone();
                    self.close_rtp_port(&old_stream_key, old_server).await;
                }

                // 分配 RTP 端口（TCP 和 UDP 都需要）
                let preferred_name = sess.read().await.media_server_name.clone();
                let preferred: Option<&str> = preferred_name.as_deref();
                let (rtp_port, server_ip, server_name) = match self.alloc_rtp_port(&stream_key, preferred).await {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::error!("[RTSP] SETUP RTP alloc failed: {}", e);
                        return (
                            events,
                            Some(RtspResponse::error(cseq, 500, "RTP allocation failed")),
                        );
                    }
                };

                {
                    let mut s = sess.write().await;
                    s.update_activity();
                    s.state = RtspSessionState::Ready;
                    s.rtp_port = Some(rtp_port);
                    s.media_server_name = Some(server_name.clone());
                }

                let transport = if wants_interleaved {
                    // TCP Interleaved 模式：注册 RTP 隧道
                    let interleaved_channels = transport_in
                        .and_then(|t| Self::parse_interleaved_channels(t))
                        .unwrap_or((0, 1));

                    let _ = self
                        .deps
                        .rtp_tunnel
                        .register(
                            stream_key.clone(),
                            format!("{}:{}", server_ip, rtp_port)
                                .parse()
                                .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
                            interleaved_channels.0,
                        )
                        .await;

                    tracing::info!(
                        "[RTSP] SETUP (TCP): stream={}, ch={}-{}, zlm={}:{}",
                        stream_key,
                        interleaved_channels.0,
                        interleaved_channels.1,
                        server_ip,
                        rtp_port
                    );

                    format!(
                        "RTP/AVP/TCP;unicast;interleaved={}-{}",
                        interleaved_channels.0, interleaved_channels.1
                    )
                } else {
                    // UDP 模式：客户端直接发送 RTP 到 ZLMediaKit
                    tracing::info!(
                        "[RTSP] SETUP (UDP): stream={}, zlm={}:{}-{}, client={}-{}",
                        stream_key,
                        server_ip,
                        rtp_port,
                        rtp_port + 1,
                        client_rtp,
                        client_rtcp
                    );

                    format!(
                        "RTP/AVP;unicast;client_port={}-{};server_port={}-{};source={}",
                        client_rtp,
                        client_rtcp,
                        rtp_port,
                        rtp_port + 1,
                        server_ip
                    )
                };

                let sess_id_str = self.session.as_deref().unwrap_or("unknown");
                tracing::info!(
                    "[RTSP] SETUP: stream={}, transport={}",
                    stream_key,
                    transport
                );
                (
                    events,
                    Some(RtspResponse::setup(cseq, sess_id_str, &transport, None)),
                )
            }

            "PLAY" => {
                let stream_key = Self::extract_stream_key(url);
                let sess_id = session_header.as_deref().unwrap_or("").trim_start_matches(';');

                let Some(sess) = get_session(sess_id) else {
                    tracing::warn!("[RTSP] PLAY: session not found: {}", sess_id);
                    return (events, Some(RtspResponse::error(cseq, 454, "Session Not Found")));
                };
                {
                    let mut s = sess.write().await;
                    s.update_activity();
                    s.state = RtspSessionState::Playing;
                }

                let rtp_info = format!(
                    "url=rtsp://{}/{};seq=0;rtptime=0",
                    self.deps.config.server.host, stream_key
                );

                let session_id = self
                    .session
                    .clone()
                    .unwrap_or_else(|| format!("rtsp_{}", chrono::Utc::now().timestamp_millis()));

                tracing::info!("[RTSP] PLAY: stream={}", stream_key);

                let media_server_name = sess.read().await.media_server_name.clone();
                events.push(SignalEvent::StartPlay {
                    device_id: 0,
                    device_tag: None,
                    session_id: session_id.clone(),
                    channel_id: Some(stream_key),
                    transport: TransportType::TCP,
                    media_server_name,
                });

                (
                    events,
                    Some(RtspResponse::play(
                        cseq,
                        self.session.as_deref().unwrap_or(""),
                        None,
                        Some(&rtp_info),
                    )),
                )
            }

            "PAUSE" => {
                if let Some(sess_id) = session_header.as_ref() {
                    if let Some(sess) = get_session(sess_id.trim_start_matches(';')) {
                        let mut s = sess.write().await;
                        s.update_activity();
                        s.state = RtspSessionState::Paused;
                    }
                }
                (events, Some(RtspResponse::ok(cseq)))
            }

            "GET_PARAMETER" => {
                if let Some(sess_id) = session_header.as_ref() {
                    if let Some(sess) = get_session(sess_id.trim_start_matches(';')) {
                        sess.write().await.update_activity();
                    }
                }
                (
                    events,
                    Some(RtspResponse::get_parameter(
                        cseq,
                        self.session.as_deref().unwrap_or(""),
                    )),
                )
            }

            "TEARDOWN" => {
                let sess_id = session_header
                    .as_deref()
                    .unwrap_or("")
                    .trim_start_matches(';');

                let (stream_key, media_server_name) =
                    if let Some(sess) = get_session(sess_id) {
                        let s = sess.read().await;
                        (s.stream_key.clone(), s.media_server_name.clone())
                    } else {
                        (sess_id.to_string(), None)
                    };

                if let Some(ref server_tag) = media_server_name {
                    self.close_rtp_port(&stream_key, server_tag).await;
                }

                self.deps.rtp_tunnel.unregister(&stream_key);

                let session_id = self.session.clone().unwrap_or(sess_id.to_string());
                events.push(SignalEvent::StopPlay {
                    device_id: 0,
                    device_tag: None,
                    session_id,
                });

                remove_session(sess_id);
                self.session = None;

                tracing::info!("[RTSP] TEARDOWN: stream={}", stream_key);
                (events, Some(RtspResponse::ok(cseq)))
            }

            _ => {
                tracing::warn!("[RTSP] Unsupported method: {}", method);
                (
                    events,
                    Some(RtspResponse::error(cseq, 405, "Method Not Allowed")),
                )
            }
        }
    }

    async fn process_message(&mut self, data: &[u8]) -> (Vec<SignalEvent>, Option<String>) {
        if data.first() == Some(&0x24) {
            return self.handle_interleaved_data(data).await;
        }

        let data_str = String::from_utf8_lossy(data);
        let lines: Vec<&str> = data_str.lines().collect();
        if lines.is_empty() {
            return (vec![], None);
        }

        let (method, url, _version) = match Self::parse_request_line(lines[0]) {
            Some(r) => r,
            None => return (vec![], None),
        };

        let headers = Self::parse_headers(&lines[1..]);

        let header_end = data_str.find("\r\n\r\n").unwrap_or(0);
        let content_length = data_str[..header_end]
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split_once(':')?.1.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let body = if content_length > 0 {
            data_str[header_end + 4..header_end + 4 + content_length].to_string()
        } else {
            String::new()
        };

        self.handle_request(&method, &url, &headers, &body).await
    }
    
    async fn send_teardown(&self, stream_key: &str) -> Result<()> {
        let cseq = self.cseq + 1;
        let remote_host = self.remote_addr
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        
        let teardown = format!(
            "TEARDOWN rtsp://{}/{} RTSP/1.0\r\n\
             CSeq: {}\r\n\
             Session: {}\r\n\
             User-Agent: RustCam-Media/2.0\r\n\r\n",
            remote_host,
            stream_key,
            cseq,
            self.session.as_deref().unwrap_or("")
        );
        
        tracing::info!("[RTSP] Sending TEARDOWN for stream: {}", stream_key);
        self.send_raw(teardown.as_bytes()).await
    }
}

// ═══════════════════════════════════════════════════════════════
// SignalAdapter Trait Implementation
// ═══════════════════════════════════════════════════════════════

#[async_trait]
impl SignalAdapter for RtspServerAdapter {
    async fn parse(&mut self, data: &[u8]) -> Result<Vec<SignalEvent>> {
        self.recv_buffer.extend_from_slice(data);
        let mut events = Vec::new();

        loop {
            let (msg, remainder) = match Self::extract_rtsp_message(&self.recv_buffer) {
                Some((msg, rem)) => (msg, rem),
                None => {
                    if self.recv_buffer.first() == Some(&0x24) {
                        let (evts, _) = self.process_message(&self.recv_buffer.clone()).await;
                        events.extend(evts);
                        self.recv_buffer.clear();
                    }
                    break;
                }
            };
            self.recv_buffer = remainder;

            let (msg_events, response) = self.process_message(&msg).await;
            events.extend(msg_events);

            if let Some(resp) = response {
                let _ = self.send_raw(resp.as_bytes()).await;
            }
        }

        Ok(events)
    }

    async fn on_connected(&mut self, addr: SocketAddr) -> Result<()> {
        self.remote_addr = Some(addr);
        tracing::info!("[RTSP] Client connected: {}", addr);
        Ok(())
    }

    async fn on_disconnected(&mut self) -> Result<()> {
        if let Some(ref sess_id) = self.session {
            tracing::info!("[RTSP] Client disconnected, session: {}", sess_id);

            let stream_key = if let Some(sess) = get_session(sess_id) {
                let s = sess.read().await;
                s.stream_key.clone()
            } else {
                sess_id.clone()
            };

            self.send_teardown(&stream_key).await?;

            self.deps.rtp_tunnel.unregister(&stream_key);
            remove_session(sess_id);
            self.session = None;
        }
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.send_raw(data).await
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Rtsp
    }

    fn name(&self) -> &'static str {
        "RTSP-Server"
    }

    fn keepalive(&self) -> bool {
        true
    }

    fn idle_timeout(&self) -> Option<u64> {
        Some(60)
    }

    fn set_tcp_write(&mut self, write: OwnedWriteHalf) {
        self.write = Some(Arc::new(tokio::sync::RwLock::new(write)));
    }

    async fn start(&mut self, _device_tag: &str) -> Result<()> { Ok(()) }
    async fn ptz_control(&mut self, _channel_id: &str, _command: &crate::protocol::event::PtzCommand, _speed: Option<u8>) -> Result<()> { Ok(()) }
}