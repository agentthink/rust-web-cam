use std::collections::HashMap;
use std::sync::Arc;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use async_trait::async_trait;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::{AsyncWriteExt, AsyncWrite};
use chrono::{DateTime, Utc};

use crate::context::registry;
use crate::protocol::event::{EventHandler, SignalEvent, ProtocolType, TransportType, CatalogChannel, PtzCommand};

use crate::adapter::media_server::RtpTransport;
use crate::protocol::adapter::SignalAdapter;
use crate::protocol::traits::ProtocolDeps;
use crate::protocol::gb28181::auth::{
    parse_sip_authorization, verify_sip_digest,
    generate_nonce, verify_nonce, consume_nonce, VerifyResult,
};
use crate::protocol::gb28181::sip::{SipMessage, SipUri, SipMethod, SipNameAddr};
use crate::error::{Result, AppError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gb28181Version { Gb2016, Gb2011 }

impl Default for Gb28181Version {
    fn default() -> Self { Gb28181Version::Gb2016 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportProtocol { Tcp, Udp }

type GbWriteHalf = Arc<tokio::sync::RwLock<OwnedWriteHalf>>;

#[derive(Clone)]
pub struct SubSession {
    pub call_id: String,
    pub seq: u32,
    pub channel_id: String,
    pub parent_device_id: String,
    pub stream_key: String,
    pub media_server_name: String,
    pub from_uri: String,
    pub to_uri: String,
    pub from_tag: String,
    pub to_tag: Option<String>,
    pub via_branch: String,
    pub server_ip: String,
    pub server_port: u16,
    pub transport: TransportProtocol,
    pub device_host: String,
    pub device_port: u16,
    pub rtp_port: u16,
    pub media_started: bool,
    pub device_nated_ip: Option<String>,
    pub device_nated_port: Option<u16>,
}

#[derive(Clone)]
 pub struct Gb28181Adapter {
    pub version: Gb28181Version,
    pub device_tag: Option<String>,
    pub from_uri: Option<String>,
    pub device_id: Option<i64>,
    pub device_name: Option<String>,
    pub seq: Arc<AtomicU32>,
    pub remote_addr: Option<SocketAddr>,
    pub recv_buffer: Arc<Vec<u8>>,
    pub write: Option<GbWriteHalf>,
    pub catalog_retry_count: u32,
    pub max_buffer_size: usize,
    pub transport: TransportProtocol,
    pub call_id: Option<String>,
    pub from_tag: Option<String>,
    pub to_tag: Option<String>,
    pub udp_peer: Option<SocketAddr>,
    pub registration_expires: Option<u64>,
    pub deps: Arc<ProtocolDeps>,
    pub sessions: Arc<tokio::sync::RwLock<HashMap<String, SubSession>>>,
    pub subscriptions: Arc<tokio::sync::RwLock<HashMap<String, Subscription>>>,
    pub audio_talk_sessions: Arc<tokio::sync::RwLock<HashMap<String, AudioTalkSession>>>,
    pub pending_catalog_query: bool,
    pub event_handler_name: Option<String>,
    pub register_fn: Arc<dyn Fn(String, crate::protocol::adapter_manager::AdapterEntry) + Send + Sync>,
    pub unregister_fn: Arc<dyn Fn(String) + Send + Sync>,
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_registered: bool,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub event_type: String,
    pub expires: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct AudioTalkSession {
    pub device_tag: String,
    pub device_ip: String,
    pub device_audio_port: u16,
    pub platform_audio_port: u16,
    pub call_id: String,
    pub from_tag: String,
    pub to_tag: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub audio_socket: Option<Arc<tokio::net::UdpSocket>>,
    pub tcp_stream: Option<Arc<tokio::sync::RwLock<tokio::net::TcpStream>>>,
    pub rtp_ssrc: u32,
    pub rtp_sequence: u16,
    pub rtp_timestamp: u32,
}

impl Gb28181Adapter {
    async fn get_device_app(&self, channel_id: &str) -> Option<String> {
        self.deps.device_lookup.find_by_tag(channel_id).await.and_then(|d| d.app)
    }

    pub fn new(
        deps: ProtocolDeps,
        register_fn: Arc<dyn Fn(String, crate::protocol::adapter_manager::AdapterEntry) + Send + Sync>,
        unregister_fn: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Self {
        Self {
            version: Gb28181Version::default(),
            device_tag: None,
            from_uri: None,
            device_id: None,
            device_name: None,
            seq: Arc::new(AtomicU32::new(0)),
            remote_addr: None,
            recv_buffer: Arc::new(Vec::new()),
            write: None,
            catalog_retry_count: 0,
            max_buffer_size: 1024 * 1024,
            transport: TransportProtocol::Tcp,
            call_id: None,
            from_tag: None,
            to_tag: None,
            udp_peer: None,
            registration_expires: None,
            deps: Arc::new(deps),
            sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            subscriptions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            audio_talk_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            pending_catalog_query: false,
            event_handler_name: None,
            register_fn,
            unregister_fn,
            last_message_at: None,
            is_registered: false,
        }
    }

    pub fn registration_expires(&self) -> u64 {
        self.registration_expires.unwrap_or(3600)
    }

    fn truncate_20(s: &str) -> String {
        if s.len() <= 20 { s.to_string() } else { s[..20].to_string() }
    }

    fn truncate_32(s: &str) -> String {
        if s.len() <= 32 { s.to_string() } else { s[..32].to_string() }
    }

    fn is_valid_gb28181_device_id(id: &str) -> bool {
        let len = id.len();
        if len < 10 || len > 20 {
            return false;
        }
        id.chars().all(|c| c.is_ascii_digit())
    }

    fn parse_ssrc_from_sdp(sdp: &str) -> Option<u32> {
        for line in sdp.lines() {
            let line = line.trim();
            if line.starts_with("y=") {
                let ssrc_str = &line[2..];
                return ssrc_str.parse::<u32>().ok();
            }
        }
        None
    }

    fn parse_subject_device_id(subject: &str) -> Option<String> {
        let parts: Vec<&str> = subject.split(':').collect();
        if parts.len() >= 2 {
            let device_id = parts[0];
            if Self::is_valid_gb28181_device_id(device_id) {
                return Some(device_id.to_string());
            }
        }
        None
    }

    fn parse_sip_message(buffer: &[u8]) -> Option<(String, Vec<(String, String)>, String)> {
        let msg_str = String::from_utf8_lossy(buffer);
        let header_end = msg_str.find("\r\n\r\n")?;
        let header_section = &msg_str[..header_end];

        let mut headers = Vec::new();
        for line in header_section.lines() {
            if let Some((k, v)) = line.split_once(':') {
                headers.push((k.trim().to_lowercase(), v.trim().to_string()));
            }
        }

        let content_length = Self::get_header(&headers, "Content-Length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let body_start = header_end + 4;
        let total_len = body_start + content_length;
        if buffer.len() < total_len { return None; }

        let first_line = header_section.lines().next()?;
        let body = if content_length > 0 { &msg_str[body_start..total_len] } else { "" };

        Some((first_line.to_string(), headers, body.to_string()))
    }

    fn get_header(headers: &[(String, String)], name: &str) -> Option<String> {
        headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    }

    fn parse_sip_uri(uri: &str) -> String {
        let uri = uri.trim();
        let uri = uri.strip_prefix('<').unwrap_or(uri).trim_end_matches('>').trim();
        let uri = uri.strip_prefix("sip:").or_else(|| uri.strip_prefix("SIP:")).unwrap_or(uri);
        uri.split('@').next().unwrap_or(uri).trim().to_string()
    }

    fn parse_sip_uri_with_domain(uri: &str) -> String {
        let uri = uri.trim();
        let uri = uri.strip_prefix('<').unwrap_or(uri).trim_end_matches('>').trim();
        if uri.to_lowercase().starts_with("sip:") {
            uri.to_string()
        } else {
            format!("sip:{}", uri)
        }
    }

    fn parse_sip_uri_with_domain_only(uri: &str) -> String {
        let uri = uri.trim();
        let uri = uri.strip_prefix('<').unwrap_or(uri);
        if let Some(angle_end) = uri.find('>') {
            let uri_part = &uri[..angle_end];
            if uri_part.to_lowercase().starts_with("sip:") {
                return uri_part.to_string();
            } else {
                return format!("sip:{}", uri_part);
            }
        }
        let uri = uri.split(';').next().unwrap_or(uri).trim();
        if uri.to_lowercase().starts_with("sip:") {
            uri.to_string()
        } else {
            format!("sip:{}", uri)
        }
    }

    fn parse_tag_from_header(header_value: &str) -> Option<String> {
        if let Some(tag_start) = header_value.find("tag=") {
            let after_tag = &header_value[tag_start + 4..];
            let end = after_tag.find(&[',', ';', '\r', '\n'][..]).unwrap_or(after_tag.len());
            if end > 0 { return Some(after_tag[..end].to_string()); }
        }
        None
    }

    fn parse_via_branch(headers: &[(String, String)]) -> String {
        Self::get_header(headers, "Via")
            .and_then(|v| {
                for part in v.split(';') {
                    let trimmed = part.trim();
                    if trimmed.starts_with("branch=") {
                        return Some(trimmed[7..].to_string());
                    }
                }
                None
            })
            .unwrap_or_else(|| "z9hG4bKdefault".to_string())
    }

    fn parse_via_received(headers: &[(String, String)]) -> Option<String> {
        Self::get_header(headers, "Via").and_then(|v| {
            for part in v.split(';') {
                let trimmed = part.trim();
                if trimmed.starts_with("received=") {
                    return Some(trimmed[9..].to_string());
                }
            }
            None
        })
    }

    fn parse_via_rport(headers: &[(String, String)]) -> Option<u16> {
        Self::get_header(headers, "Via").and_then(|v| {
            for part in v.split(';') {
                let trimmed = part.trim();
                if trimmed.starts_with("rport") {
                    if trimmed == "rport" {
                        return Some(0);
                    }
                    if let Some(val) = trimmed.strip_prefix("rport=") {
                        return val.parse().ok();
                    }
                }
            }
            None
        })
    }

    fn parse_cseq_line(headers: &[(String, String)]) -> String {
        Self::get_header(headers, "CSeq").unwrap_or_else(|| "1 REGISTER".to_string())
    }

    fn parse_xml_field(xml: &str, tag: &str) -> Option<String> {
        let start = format!("<{}>", tag);
        let end = format!("</{}>", tag);
        if let Some(s) = xml.find(&start) {
            let data_start = s + start.len();
            if let Some(e) = xml[data_start..].find(&end) {
                return Some(xml[data_start..data_start + e].trim().to_string());
            }
        }
        None
    }

    fn parse_channel_id(body: &str) -> Option<String> {
        Self::parse_xml_field(body, "ChannelID")
            .or_else(|| Self::parse_xml_field(body, "DeviceID"))
    }

    fn best_direction(pan: f64, tilt: f64, zoom: f64) -> &'static str {
        let p = pan.abs();
        let t = tilt.abs();
        let z = zoom.abs();
        if p >= t && p >= z {
            if pan > 0.0 { "right" } else { "left" }
        } else if t >= p && t >= z {
            if tilt > 0.0 { "up" } else { "down" }
        } else {
            if zoom > 0.0 { "zoom_in" } else { "zoom_out" }
        }
    }

    fn encode_ptz_cmd_compound(pan: f64, tilt: f64, zoom: f64, base_speed: u8) -> String {
        let pan_speed = ((pan.abs() * 255.0) as u8).max(1);
        let tilt_speed = ((tilt.abs() * 255.0) as u8).max(1);
        let zoom_speed = ((zoom.abs() * 255.0) as u8).max(1);

        let cmd1: u8 = 0x00;
        let cmd1 = if tilt > 0.01 { cmd1 | 0x08 } else { cmd1 };
        let cmd1 = if tilt < -0.01 { cmd1 | 0x10 } else { cmd1 };
        let cmd1 = if pan < -0.01 { cmd1 | 0x02 } else { cmd1 };
        let cmd1 = if pan > 0.01 { cmd1 | 0x20 } else { cmd1 };

        let cmd2: u8 = 0x00;
        let cmd2 = if zoom > 0.01 { cmd2 | 0x80 } else { cmd2 };
        let cmd2 = if zoom < -0.01 { cmd2 | 0x40 } else { cmd2 };

        let p_byte = if pan.abs() > 0.01 { pan_speed } else { 0 };
        let t_byte = if tilt.abs() > 0.01 { tilt_speed } else { 0 };
        let z_byte = if zoom.abs() > 0.01 { zoom_speed } else { 0 };

        format!("80 01 {:02X} {:02X} {:02X} {:02X} {:02X} 00", cmd1, cmd2, p_byte, t_byte, z_byte)
    }

    fn encode_ptz_cmd(direction: &str, speed: u8) -> String {
        let spd = ((speed as f64 / 100.0) * 255.0) as u8;
        match direction {
            "up" => format!("80 01 04 08 00 {:02X} 00 {:02X} 00 00 00 00", spd, spd),
            "down" => format!("80 01 04 10 00 {:02X} 00 {:02X} 00 00 00 00", spd, spd),
            "left" => format!("80 01 04 20 00 {:02X} 00 {:02X} 00 00 00 00", spd, spd),
            "right" => format!("80 01 04 40 00 {:02X} 00 {:02X} 00 00 00 00", spd, spd),
            "zoom_in" => format!("80 01 04 80 00 {:02X} 00 00 00 00 00 00", spd),
            "zoom_out" => format!("80 01 04 00 01 {:02X} 00 00 00 00 00 00", spd),
            "stop" => "80 01 04 00 00 00 00 00 00 00 00 00".to_string(),
            _ => "80 01 04 00 00 00 00 00 00 00 00 00".to_string(),
        }
    }

    fn build_ptz_xml(channel_id: &str, direction: &str, speed: u8) -> String {
        let ptz_cmd = Self::encode_ptz_cmd(direction, speed);
        let sn = chrono::Utc::now().timestamp_millis() % 1000000;
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>PTZCmd</CmdType><SN>{}</SN><DeviceID>{}</DeviceID><PTZCmd>{}</PTZCmd></Notify>"#,
            sn, channel_id, ptz_cmd
        )
    }

    fn build_ptz_xml_compound(channel_id: &str, pan: f64, tilt: f64, zoom: f64, base_speed: u8) -> String {
        let ptz_cmd = Self::encode_ptz_cmd_compound(pan, tilt, zoom, base_speed);
        let sn = chrono::Utc::now().timestamp_millis() % 1000000;
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>PTZCmd</CmdType><SN>{}</SN><DeviceID>{}</DeviceID><PTZCmd>{}</PTZCmd></Notify>"#,
            sn, channel_id, ptz_cmd
        )
    }

    fn build_ptz_message(
        &self, channel_id: &str, parent_device_id: &str, body: &str,
    ) -> String {
        let sn = self.seq.fetch_add(1, Ordering::SeqCst);
        let call_id = uuid::Uuid::new_v4().to_string().replace("-", "");
        let server_port = self.deps.config.server.port;
        let server_ip = self.remote_addr.map(|a| a.ip().to_string()).unwrap_or_else(|| "0.0.0.0".to_string());
        let branch = format!("z9hG4bK{}", &call_id[..7]);

        let via = if self.transport == TransportProtocol::Tcp {
            format!("SIP/2.0/TCP {}:{};branch={};rport", server_ip, server_port, branch)
        } else {
            format!("SIP/2.0/UDP {}:{};branch={};rport", server_ip, server_port, branch)
        };

        let body_len = body.len();
        let to_uri = format!("sip:{}@{}", parent_device_id,
            self.remote_addr.map(|a| a.to_string()).unwrap_or_else(|| "0.0.0.0:5060".to_string()));
        let from_uri = format!("sip:rustcam@{}:{}", server_ip, server_port);
        let from_tag = uuid::Uuid::new_v4().to_string().replace("-", "");

        format!(
            "MESSAGE {to_uri} SIP/2.0\r\n\
             Via: {via}\r\n\
             From: <{from_uri}>;tag={from_tag}\r\n\
             To: <{to_uri}>\r\n\
             Call-ID: {call_id}\r\n\
             CSeq: {sn} MESSAGE\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Content-Type: Application/MANSCDP+xml\r\n\
             Content-Length: {body_len}\r\n\
             \r\n\
             {body}",
            to_uri = to_uri,
            via = via,
            from_uri = from_uri,
            from_tag = from_tag,
            call_id = call_id,
            sn = sn,
            body_len = body_len,
            body = body
        )
    }

    fn build_catalog_query(&self) -> Vec<u8> {
        let sn = self.seq.load(Ordering::SeqCst);
        let device_tag = self.device_tag_str();
        let (platform_id, platform_domain, platform_ip, platform_port) = crate::protocol::gb28181::get_gb28181_platform_config()
            .get()
            .map(|(id, domain, ip, port)| (id.as_str(), domain.as_str(), ip.as_str(), *port))
            .unwrap_or(("00000000000000000000", "0000000000", "0.0.0.0", 5060));

        let local_ip = if platform_ip == "0.0.0.0" {
            if let Some(peer) = self.udp_peer {
                crate::protocol::gb28181::detect_local_ip(peer.ip().to_string().as_str())
                    .unwrap_or_else(|| "0.0.0.0".to_string())
            } else {
                platform_ip.to_string()
            }
        } else {
            platform_ip.to_string()
        };

        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Query><CmdType>Catalog</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<Info><DeviceID>{}</DeviceID></Info></Query>"#,
            sn, device_tag, device_tag
        );

        let ts = chrono::Utc::now().timestamp_millis() % 1000000;
        let call_id = format!("cat-{}", ts);
        let from_uri = SipUri {
            user: Some(platform_id.to_string()),
            host: format!("{}:{}", local_ip, platform_port),
            port: None,
            params: HashMap::new(),
        };
        let device_ip = self.udp_peer
            .map(|p| p.ip().to_string())
            .unwrap_or_else(|| local_ip.clone());
        let to_uri = SipUri {
            user: Some(device_tag.to_string()),
            host: format!("{}:{}", device_ip, platform_port),
            port: None,
            params: HashMap::new(),
        };

        let from_name_addr = SipNameAddr::new(from_uri)
            .with_param("domain", platform_domain)
            .with_param("tag", &format!("cat{}", ts));
        let to_name_addr = SipNameAddr::new(to_uri.clone());

        let via_str = format!("SIP/2.0/UDP {}:{};branch=z9hG4bKcat{}", local_ip, platform_port, ts);
        
        let msg = SipMessage::request(SipMethod::Message, to_uri)
            .header("via", &via_str)
            .header("from", &from_name_addr.to_string())
            .header("to", &to_name_addr.to_string())
            .header("call-id", &call_id)
            .header("CSeq", &format!("{} MESSAGE", sn))
            .header("content-type", "Application/MANSCDP+xml")
            .header("user-agent", "RustCam-Media/2.0")
            .set_body(&body);

        msg.to_string().into_bytes()
    }

    fn parse_catalog_response(&self, body: &str) -> Vec<CatalogChannel> {
        let mut channels = Vec::new();
        let mut start = 0;
        while let Some(item_start) = body[start..].find("<Item>") {
            let abs_start = start + item_start;
            if let Some(item_end) = body[abs_start..].find("</Item>") {
                let item_str = &body[abs_start..abs_start + item_end + 7];

                let device_id = Self::parse_xml_field(item_str, "DeviceID").unwrap_or_default();
                if !device_id.is_empty() {
                    let parental = Self::parse_xml_field(item_str, "Parental")
                        .and_then(|v| v.parse::<u32>().ok())
                        .map(|v| v > 0)
                        .unwrap_or(false);

                    let port = Self::parse_xml_field(item_str, "Port")
                        .and_then(|v| v.parse::<u16>().ok());

                    let secrecy = Self::parse_xml_field(item_str, "Secrecy")
                        .and_then(|v| v.parse::<u8>().ok());

                    let ptz_type = Self::parse_xml_field(item_str, "PTZType")
                        .and_then(|v| v.parse::<u8>().ok());

                    let info = self.parse_device_info_block(item_str);

                    channels.push(CatalogChannel {
                        device_id,
                        name: Self::parse_xml_field(item_str, "Name").unwrap_or_default(),
                        manufacturer: Self::parse_xml_field(item_str, "Manufacturer"),
                        model: Self::parse_xml_field(item_str, "Model"),
                        status: Self::parse_xml_field(item_str, "Status").unwrap_or_else(|| "OFF".to_string()),
                        parental,
                        parent_id: Some(self.device_tag_str()),
                        civil_code: Self::parse_xml_field(item_str, "CivilCode"),
                        address: Self::parse_xml_field(item_str, "Address"),
                        ip_address: Self::parse_xml_field(item_str, "IPAddress"),
                        port,
                        owner: Self::parse_xml_field(item_str, "Owner"),
                        secrecy,
                        device_type: Self::parse_xml_field(item_str, "DeviceType"),
                        ptz_type,
                        info,
                    });
                }
                start = abs_start + item_end + 7;
            } else {
                break;
            }
        }

        channels
    }

    fn parse_device_info_block(&self, xml: &str) -> Option<crate::protocol::event::DeviceInfoBlock> {
        let info_start = xml.find("<Info>")?;
        let info_end = xml.find("</Info>")?;
        let info_str = &xml[info_start + 6..info_end];

        Some(crate::protocol::event::DeviceInfoBlock {
            device_type: Self::parse_xml_field(info_str, "DeviceType"),
            protocol: Self::parse_xml_field(info_str, "Protocol"),
            ptz_type: Self::parse_xml_field(info_str, "PTZType").and_then(|v| v.parse::<u8>().ok()),
            video_input_number: Self::parse_xml_field(info_str, "VideoInputNumber").and_then(|v| v.parse::<u8>().ok()),
            audio_input_number: Self::parse_xml_field(info_str, "AudioInputNumber").and_then(|v| v.parse::<u8>().ok()),
            alarm_output_number: Self::parse_xml_field(info_str, "AlarmOutputNumber").and_then(|v| v.parse::<u8>().ok()),
        })
    }

    fn detect_version(body: &str) -> Gb28181Version {
        if body.contains("GB28181") || body.contains("28181") { Gb28181Version::Gb2016 }
        else { Gb28181Version::Gb2016 }
    }

    fn via_header(&self, branch: &str) -> String {
        let addr = self.remote_addr.map(|a| a.to_string()).unwrap_or_else(|| "0.0.0.0".to_string());
        match self.transport {
            TransportProtocol::Tcp => format!("SIP/2.0/TCP {};rport;branch={}", addr, branch),
            TransportProtocol::Udp => format!("SIP/2.0/UDP {};rport;branch={}", addr, branch),
        }
    }

    fn call_id_str(&self) -> String {
        self.call_id.clone().unwrap_or_else(|| "rustcam-call-id".to_string())
    }

    fn from_tag_param(&self) -> String {
        self.from_tag.as_ref().map(|t| format!(";tag={}", t)).unwrap_or_default()
    }

    fn to_tag_param(&self) -> String {
        self.to_tag.as_ref().map(|t| format!(";tag={}", t)).unwrap_or_default()
    }

    fn build_sip_response_only_headers(
        &self, status_code: u16, status_text: &str,
        via_branch: &str, cseq_line: &str,
    ) -> String {
        let device = self.device_tag_str();
        let expires_line = if let Some(exp) = self.registration_expires {
            format!("Expires: {}\r\n", exp)
        } else {
            String::new()
        };
        let from_uri = self.from_uri.clone().unwrap_or_else(|| format!("sip:{}", self.device_tag_str()));
        format!(
            "SIP/2.0 {} {}\r\n\
             Via: {}\r\n\
             From: <{}>{}\r\n\
             To: <{}>{}\r\n\
             CSeq: {}\r\n\
             Call-ID: {}\r\n\
             {}User-Agent: RustCam-Media/2.0\r\n\
             Content-Length: 0\r\n\r\n",
            status_code, status_text,
            self.via_header(via_branch),
            from_uri, self.from_tag_param(),
            from_uri, self.to_tag_param(),
            cseq_line, self.call_id_str(),
            expires_line,
        )
    }

    fn build_401_response(&self, via_branch: &str, cseq_line: &str, realm: &str) -> String {
        let nonce = generate_nonce();
        let from_uri = self.from_uri.clone().unwrap_or_else(|| {
            let device = self.device_tag_str();
            if device.starts_with("sip:") || device.starts_with("SIP:") {
                device
            } else {
                format!("sip:{}", device)
            }
        });
        tracing::info!("[GB28181] build_401_response: from_uri='{}', from_tag='{}', to_tag='{}'", 
            from_uri, self.from_tag.as_deref().unwrap_or(""), self.to_tag.as_deref().unwrap_or(""));
        let www_auth = format!(r#"Digest realm="{}", nonce="{}""#, realm, nonce);
        let response = format!(
            "SIP/2.0 401 Unauthorized\r\n\
             Via: {}\r\n\
             From: <{}>{}\r\n\
             To: <{}>{}\r\n\
             CSeq: {}\r\n\
             Call-ID: {}\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             WWW-Authenticate: {}\r\n\
             Content-Length: 0\r\n\r\n",
            self.via_header(via_branch),
            from_uri, self.from_tag_param(),
            from_uri, self.to_tag_param(),
            cseq_line, self.call_id_str(), www_auth
        );
        tracing::info!("[GB28181] 401 response:\n{}", response);
        response
    }

    fn build_403_response(&self, via_branch: &str, cseq_line: &str) -> String {
        let from_uri = self.from_uri.clone().unwrap_or_else(|| {
            let device = self.device_tag_str();
            if device.starts_with("sip:") || device.starts_with("SIP:") {
                device
            } else {
                format!("sip:{}", device)
            }
        });
        format!(
            "SIP/2.0 403 Forbidden\r\n\
             Via: {}\r\n\
             From: <{}>{}\r\n\
             To: <{}>{}\r\n\
             CSeq: {}\r\n\
             Call-ID: {}\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Content-Length: 0\r\n\r\n",
            self.via_header(via_branch),
            from_uri, self.from_tag_param(),
            from_uri, self.to_tag_param(),
            cseq_line, self.call_id_str()
        )
    }
    
    fn build_403_response_with_headers(&self, via_branch: &str, cseq_line: &str, from_hdr: &str, to_hdr: &str, call_id_hdr: &str) -> String {
        let from_tag = Self::parse_tag_from_header(from_hdr).map(|t| format!(";tag={}", t)).unwrap_or_default();
        let to_tag = Self::parse_tag_from_header(to_hdr).map(|t| format!(";tag={}", t)).unwrap_or_default();
        format!(
            "SIP/2.0 403 Forbidden\r\n\
             Via: {}\r\n\
             From: {}{}\r\n\
             To: {}{}\r\n\
             CSeq: {}\r\n\
             Call-ID: {}\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Content-Length: 0\r\n\r\n",
            self.via_header(via_branch),
            from_hdr, from_tag,
            to_hdr, to_tag,
            cseq_line, call_id_hdr
        )
    }

    fn build_device_info_response(&self) -> String {
        let device_tag = self.device_tag_str();
        let device_name = self.device_name.as_deref().unwrap_or("IPC");
        let sn = self.seq.load(Ordering::SeqCst);
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>DeviceInfo</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<DeviceName>{}</DeviceName><Result>OK</Result><Manufacturer>RustCam</Manufacturer>
<Model>RustCam-GB28181</Model><Firmware>1.0.0</Firmware><Channel>1</Channel>
<Online>1</Online><Status>OK</Status></Notify>"#,
            sn, device_tag, device_name
        )
    }

    fn build_device_status_response(&self) -> String {
        let device_tag = self.device_tag_str();
        let sn = self.seq.load(Ordering::SeqCst);
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>DeviceStatus</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<Online>ON</Online><Status>OK</Status></Notify>"#,
            sn, device_tag
        )
    }

    fn build_device_config_response(&self, config_type: &str) -> String {
        let device_tag = self.device_tag_str();
        let sn = self.seq.load(Ordering::SeqCst);
        let config_item = match config_type {
            "BasicParam" => r#"<BasicParam><VideoEnable>true</VideoEnable><AudioEnable>true</AudioEnable></BasicParam>"#,
            "NetworkParam" => r#"<NetworkParam><IPAddress>0.0.0.0</IPAddress><Port>0</Port></NetworkParam>"#,
            "VideoParam" => r#"<VideoParam><Resolution>1920x1080</Resolution><BitRate>2048</BitRate><FrameRate>25</FrameRate></VideoParam>"#,
            "VideoSrcParam" => r#"<VideoSrcParam><Resolution>1920x1080</Resolution><Quality>85</Quality></VideoSrcParam>"#,
            _ => r#"<ConfigParam></ConfigParam>"#,
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>ConfigDownload</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<Result>OK</Result>{}</Notify>"#,
            sn, device_tag, config_item
        )
    }

    fn build_preset_list_response(&self, channel_id: &str) -> String {
        let sn = self.seq.load(Ordering::SeqCst);
        let presets = vec![
            ("1", "Preset 1"),
            ("2", "Preset 2"),
            ("3", "Preset 3"),
        ];
        let mut preset_items = String::new();
        for (idx, name) in presets {
            preset_items.push_str(&format!(r#"<PresetItem><id>{}</id><presetName>{}</presetName></PresetItem>"#, idx, name));
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>PresetList</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<PresetList>{}</PresetList></Notify>"#,
            sn, channel_id, preset_items
        )
    }

    fn build_preset_response(&self, channel_id: &str, preset_id: &str, preset_name: &str, cmd_type: &str) -> String {
        let sn = self.seq.load(Ordering::SeqCst);
        let result = if cmd_type == "SetPreset" {
            format!(r#"<PresetID>{}</PresetID><PresetName>{}</PresetName>"#, preset_id, preset_name)
        } else {
            String::new()
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>{}</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<Result>OK</Result>{}</Notify>"#,
            cmd_type, sn, channel_id, result
        )
    }

    fn build_sdp(
        &self,
        server_gb_id: &str,
        server_ip: &str,
        rtp_port: u16,
        ssrc: &str,
        config: Option<&crate::domain::device::StreamConfig>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> String {
        let video_codec = config.and_then(|c| c.video_codec.clone()).unwrap_or_else(|| "PS".to_string());
        let audio_codec = config.and_then(|c| c.audio_codec.clone()).unwrap_or_else(|| "PCMA".to_string());
        let stream_mode = config.and_then(|c| c.stream_mode.clone()).unwrap_or_else(|| "recvonly".to_string());
        let profile_level_id = config.and_then(|c| c.profile_level_id.clone());
        let packaging_mode = config.and_then(|c| c.packaging_mode.clone());
        let sprop_parameter_sets = config.and_then(|c| c.sprop_parameter_sets.clone());

        let video_pt = config.and_then(|c| c.video_payload_type)
            .unwrap_or_else(|| Self::codec_to_video_pt(&video_codec));
        let audio_pt = config.and_then(|c| c.audio_payload_type)
            .unwrap_or_else(|| Self::codec_to_audio_pt(&audio_codec));

        let (video_encoding, video_clock) = Self::codec_to_rtp_map(&video_codec);
        let (audio_encoding, audio_clock) = Self::codec_to_audio_rtp_map(&audio_codec);

        let mut video_line = format!("m=video {} RTP/AVP {}\r\n", rtp_port, video_pt);
        video_line.push_str(&format!("a={}\r\n", stream_mode));
        video_line.push_str(&format!("a=rtpmap:{} {}/{}\r\n", video_pt, video_encoding, video_clock));

        if let Some(ref pli) = profile_level_id {
            if video_codec == "PS" {
                if let Some(ref pm) = packaging_mode {
                    video_line.push_str(&format!("a=fmtp:{} profile-level-id={};packaging_mode={}\r\n", video_pt, pli, pm));
                }
            } else if video_codec == "H264" || video_codec == "H265" {
                if let Some(ref sps) = sprop_parameter_sets {
                    video_line.push_str(&format!("a=fmtp:{} profile-level-id={};sprop-parameter-sets={}\r\n", video_pt, pli, sps));
                } else {
                    video_line.push_str(&format!("a=fmtp:{} profile-level-id={}\r\n", video_pt, pli));
                }
            } else {
                video_line.push_str(&format!("a=fmtp:{} profile-level-id={}\r\n", video_pt, pli));
            }
        }

        let audio_enabled = !audio_codec.is_empty() && audio_codec.to_uppercase() != "NONE" && audio_codec.to_uppercase() != "OFF";
        let audio_line = if audio_enabled {
            format!(
                "m=audio {} RTP/AVP {}\r\n\
                 a={}\r\n\
                 a=rtpmap:{} {}/{}\r\n",
                rtp_port + 2,
                audio_pt,
                stream_mode,
                audio_pt,
                audio_encoding,
                audio_clock
            )
        } else {
            String::new()
        };

        let (sdp_type, time_range) = if let (Some(start), Some(end)) = (start_time, end_time) {
            ("Playback", format!("{} {}", Self::datetime_to_ntp_string(start), Self::datetime_to_ntp_string(end)))
        } else {
            ("Play", "0 0".to_string())
        };

        let mut sdp = format!(
            "v=0\r\n\
             o={} 0 0 IN IP4 {}\r\n\
             s={}\r\n\
             c=IN IP4 {}\r\n\
             t={}\r\n",
            server_gb_id, server_ip, sdp_type, server_ip, time_range
        );

        sdp.push_str(&video_line);
        sdp.push_str(&audio_line);
        sdp.push_str(&format!("y={}\r\n", ssrc));
        sdp.push_str("f=\r\n");

        sdp
    }

    fn datetime_to_ntp_string(dt: DateTime<Utc>) -> String {
        dt.format("%Y%m%d%H%M%S").to_string()
    }

    fn codec_to_video_pt(codec: &str) -> u8 {
        match codec.to_uppercase().as_str() {
            "PS" => 96,
            "H264" | "H.264" => 98,
            "H265" | "H.265" | "HEVC" => 99,
            "MPEG4" | "MP4V" => 97,
            "AVC" => 98,
            _ => 96,
        }
    }

    fn codec_to_audio_pt(codec: &str) -> u8 {
        match codec.to_uppercase().as_str() {
            "PCMA" | "G711A" => 8,
            "PCMU" | "G711U" => 0,
            "AAC" => 97,
            "G722" => 9,
            "G728" => 15,
            "G729" => 18,
            _ => 8,
        }
    }

    fn codec_to_rtp_map(codec: &str) -> (&'static str, u32) {
        match codec.to_uppercase().as_str() {
            "PS" => ("PS", 90000),
            "H264" | "H.264" | "AVC" => ("H264", 90000),
            "H265" | "H.265" | "HEVC" => ("H265", 90000),
            "MPEG4" | "MP4V" => ("MPEG4", 90000),
            _ => ("PS", 90000),
        }
    }

    fn codec_to_audio_rtp_map(codec: &str) -> (&'static str, u32) {
        match codec.to_uppercase().as_str() {
            "PCMA" | "G711A" => ("PCMA", 8000),
            "PCMU" | "G711U" => ("PCMU", 8000),
            "AAC" => ("AAC", 8000),
            "G722" => ("G722", 8000),
            "G728" => ("G728", 16000),
            "G729" => ("G729", 8000),
            _ => ("PCMA", 8000),
        }
    }

    fn build_catalog_response(&self, channel_id: &str) -> String {
        let device_tag = self.device_tag_str();
        let device_name = self.device_name.as_deref().unwrap_or("IPC");
        let sn = self.seq.load(Ordering::SeqCst);
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Response><CmdType>Catalog</CmdType><SN>{}</SN><DeviceID>{}</DeviceID><SumNum>1</SumNum>
<DeviceList><Item><DeviceID>{}</DeviceID><Name>{}</Name><DeviceType>IPC</DeviceType>
<Manufacturer>RustCam</Manufacturer><Model>IPC</Model><Owner>Owner</Owner>
<CivilCode>CivilCode</CivilCode><Address>Address</Address><Parental>0</Parental>
<SafetyWay>0</SafetyWay><RegisterWay>1</RegisterWay><CertNum>CertNum</CertNum>
<ErrCode>0</ErrCode><Secrecy>0</Secrecy><IPAddress>0.0.0.0</IPAddress><Port>0</Port>
<Password></Password><Status>ON</Status><PTZType>3</PTZType>
<Info><DeviceType>IPC</DeviceType><Protocol>GB28181</Protocol><PTZType>3</PTZType>
<VideoInputNumber>1</VideoInputNumber><AudioInputNumber>1</AudioInputNumber>
<AlarmOutputNumber>0</AlarmOutputNumber></Info></Item></DeviceList></Response>"#,
            sn, device_tag, channel_id, device_name
        )
    }

    async fn alloc_rtp_port(&self, stream_key: &str, server_name: Option<&str>) -> Result<(u16, String, String)> {
        let adapter = if let Some(name) = server_name {
            self.deps.cluster.get_server(name)
        } else {
            None
        };

        let adapter = match adapter {
            Some(a) if a.is_online().await => {
                tracing::info!("[GB28181] Using device-associated media server: {}", server_name.unwrap());
                a
            }
            _ => {
                if server_name.is_some() {
                    tracing::warn!("[GB28181] Media server {} unavailable, falling back", server_name.unwrap());
                }
                match self.deps.cluster.select_server().await {
                    Some(s) => s,
                    None => return Err(AppError::MediaServerError("No media server available".to_string())),
                }
            }
        };

        let tag = adapter.tag().to_string();
        match adapter.open_rtp_server(stream_key, 0, RtpTransport::Udp).await {
            Ok((port, ip)) => {
                tracing::info!("[GB28181] RTP allocated: stream={}, port={}, ip={}, server={}", stream_key, port, ip, tag);
                Ok((port, ip, tag))
            }
            Err(e) => {
                tracing::warn!("[GB28181] RTP failed on {}, trying fallback: {}", tag, e);
                if server_name.is_some() {
                    match self.deps.cluster.select_server().await {
                        Some(fallback) => {
                            let fb_tag = fallback.tag().to_string();
                            fallback.open_rtp_server(stream_key, 0, RtpTransport::Udp).await
                                .map(|(port, ip)| {
                                    tracing::info!("[GB28181] RTP fallback: server={}", fb_tag);
                                    (port, ip, fb_tag)
                                })
                                .map_err(|e2| AppError::MediaServerError(format!("RTP fallback failed: {}", e2)))
                        }
                        None => Err(AppError::MediaServerError(format!("No media server available: {}", e))),
                    }
                } else {
                    Err(AppError::MediaServerError(format!("No media server available: {}", e)))
                }
            }
        }
    }

    async fn handle_register(
        &mut self, headers: &[(String, String)], body: &str,
    ) -> (Option<SignalEvent>, Option<String>) {
        self.version = Self::detect_version(body);

        let device_id = Self::get_header(headers, "From")
            .or_else(|| Self::get_header(headers, "To"))
            .map(|v| Self::parse_sip_uri(&v))
            .unwrap_or_else(|| "unknown".to_string());

        tracing::info!("[GB28181] handle_register: device_id={}", device_id);
        self.device_tag = Some(device_id.clone());
        if let Some(from_hdr) = Self::get_header(headers, "From") {
            self.from_uri = Some(Self::parse_sip_uri_with_domain_only(&from_hdr));
        }
        if let Some(name) = Self::parse_xml_field(body, "DeviceName") {
            self.device_name = Some(name);
        }

        let expires = Self::get_header(headers, "Expires")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3600);
        self.registration_expires = Some(expires.clamp(60, 86400));

        if self.call_id.is_none() {
            if let Some(cid) = Self::get_header(headers, "Call-ID") { self.call_id = Some(cid); }
        }
        if let Some(from_hdr) = Self::get_header(headers, "From") {
            if self.from_tag.is_none() {
                if let Some(tag) = Self::parse_tag_from_header(&from_hdr) { self.from_tag = Some(tag); }
            }
        }
        if let Some(to_hdr) = Self::get_header(headers, "To") {
            if let Some(tag) = Self::parse_tag_from_header(&to_hdr) {
                if self.to_tag.is_none() { self.to_tag = Some(tag); }
            } else if self.to_tag.is_none() {
                self.to_tag = Some(uuid::Uuid::new_v4().to_string().replace("-", ""));
            }
        }

        let via_branch = Self::parse_via_branch(headers);
        let cseq = Self::parse_cseq_line(headers);
        tracing::info!("[GB28181] REGISTER: {} (expires={}s)", device_id, self.registration_expires());

        let auth_header = headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Authorization"))
            .map(|(_, v)| v.as_str());

        if auth_header.is_none() {
            let server_realm = super::get_gb28181_platform_config()
                .get().map(|c| c.1.clone()).unwrap_or_else(|| device_id.clone());
            tracing::info!("[GB28181] No Authorization header, sending 401 to {} with realm={}", device_id, server_realm);
            return (None, Some(self.build_401_response(&via_branch, &cseq, &server_realm)));
        }

        let auth_params = match parse_sip_authorization(auth_header.unwrap()) {
            Some(p) => p,
            None => {
                tracing::warn!("[GB28181] Failed to parse Authorization header: {}", auth_header.unwrap());
                return (None, Some(self.build_403_response(&via_branch, &cseq)));
            }
        };
        tracing::info!("[GB28181] Auth params: username={}, realm={}, nonce={}", auth_params.username, auth_params.realm, auth_params.nonce);
        if !verify_nonce(&auth_params.nonce) {
            let server_realm = super::get_gb28181_platform_config()
                .get().map(|c| c.1.clone()).unwrap_or_else(|| device_id.clone());
            tracing::warn!("[GB28181] Invalid nonce from {}, sending 401 with new nonce", device_id);
            return (None, Some(self.build_401_response(&via_branch, &cseq, &server_realm)));
        }

        let device_opt = self.deps.device_lookup.find_by_tag(&device_id).await;
        let (username, password, internal_id, device_tag) = match device_opt {
            Some(d) => (d.device_tag.clone().unwrap_or_default(), d.device_password.clone().unwrap_or_default(), d.id, Some(d.device_tag.unwrap_or_default())),
            None => {
                tracing::warn!("[GB28181] Device not found: {}", device_id);
                return (None, Some(self.build_403_response(&via_branch, &cseq)));
            }
        };
        tracing::info!("[GB28181] Device found: username={}, password={}", username, password);
        if auth_params.username != username {
            tracing::warn!("[GB28181] Username mismatch: auth={} vs db={}", auth_params.username, username);
            return (None, Some(self.build_403_response(&via_branch, &cseq)));
        }

        match verify_sip_digest(&auth_params, &password, "REGISTER") {
            VerifyResult::Valid => {
                tracing::info!("[GB28181] Device {} authenticated", device_id);
                consume_nonce(&auth_params.nonce);
                self.device_id = Some(internal_id as i64);

                let already_registered = self.is_registered;
                self.is_registered = true;

                let entry: crate::protocol::adapter_manager::AdapterEntry = Arc::new(tokio::sync::Mutex::new(Box::new(self.clone())));
                (self.register_fn)(device_id.clone(), entry);
                let _ = self.deps.device_lookup.set_online(&device_id).await;

                if !already_registered {
                    self.pending_catalog_query = true;
                }

               // let bus = registry().infra.event_bus.clone();
               // let handler_name = format!("gb28181:{}", device_id);
             //   self.event_handler_name = Some(handler_name.clone());
              //  bus.on(&handler_name, Box::new(self.clone()));
              //   tracing::info!("[GB28181] EventHandler registered as {} for {}", handler_name, device_id);

                let event = if already_registered {
                    tracing::debug!("[GB28181] Device {} re-registered (keepalive)", device_id);
                    SignalEvent::DeviceKeepalive {
                        device_id: internal_id,
                        device_tag,
                        timestamp: chrono::Utc::now(),
                    }
                } else {
                    tracing::info!("[GB28181] Device {} registered as new", device_id);
                    SignalEvent::DeviceRegister {
                        device_id: internal_id,
                        device_tag,
                        name: self.device_name.clone().unwrap_or_else(|| "GB28181 Device".to_string()),
                        stream_key: None,
                        manufacturer: Self::parse_xml_field(body, "Manufacturer"),
                        model: Self::parse_xml_field(body, "Model"),
                        protocol: ProtocolType::Gb28181,
                    }
                };

                (Some(event), Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)))
            }
            _ => (None, Some(self.build_403_response(&via_branch, &cseq))),
        }
    }
    

    async fn handle_message(
        &mut self, headers: &[(String, String)], body: &str,
    ) -> (Option<SignalEvent>, Option<String>) {
        if self.call_id.is_none() {
            if let Some(cid) = Self::get_header(headers, "Call-ID") { self.call_id = Some(cid); }
        }
        let via_branch = Self::parse_via_branch(headers);
        let cseq = Self::parse_cseq_line(headers);

        if self.device_tag.is_none() || !self.is_registered {
            let from_hdr = Self::get_header(headers, "From").unwrap_or_default();
            let device_id_from_header = Self::parse_sip_uri(&from_hdr);
            if !device_id_from_header.is_empty() {
                if let Some(device) = self.deps.device_lookup.find_by_tag(&device_id_from_header).await {
                    tracing::info!("[GB28181] Restoring registration for device {} in new adapter", device_id_from_header);
                    self.device_tag = Some(device_id_from_header.clone());
                    self.device_id = Some(device.id);
                    self.is_registered = true;
                    self.from_uri = Some(Self::parse_sip_uri_with_domain_only(&from_hdr));
                    self.pending_catalog_query = true;
                }
            }
            if !self.is_registered {
                tracing::warn!("[GB28181] Device not registered, rejecting MESSAGE");
                let from_hdr = Self::get_header(headers, "From").unwrap_or_default();
                let to_hdr = Self::get_header(headers, "To").unwrap_or_default();
                let call_id_hdr = Self::get_header(headers, "Call-ID").unwrap_or_default();
                let resp = self.build_403_response_with_headers(&via_branch, &cseq, &from_hdr, &to_hdr, &call_id_hdr);
                return (None, Some(resp));
            }
        }

        if body.contains("<CmdType>Alarm</CmdType>") || body.contains("<AlarmNotify>") || body.contains("<Alarm>") {
            let alarm_type = Self::parse_xml_field(body, "AlarmType").unwrap_or_else(|| "unknown".to_string());
            let alarm_msg = Self::parse_xml_field(body, "AlarmDescription").unwrap_or_default();
            let device_tag = self.device_tag_str();
            let device_id_val = self.device_id_i64();
            return (Some(SignalEvent::Alarm {
                device_id: device_id_val.to_string(),
                alarm_type,
                message: alarm_msg,
                timestamp: chrono::Utc::now(),
            }), Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)));
        }

        if self.pending_catalog_query && !body.contains("Catalog") {
            self.catalog_retry_count += 1;
            if self.catalog_retry_count >= 3 {
                self.pending_catalog_query = false;
                self.catalog_retry_count = 0;
                tracing::warn!("[GB28181] Catalog query retries exhausted, clearing pending flag");
            }
        }

        if body.is_empty() || body.contains("Keepalive") || body.contains("heartbeat") {
            let device_tag = self.device_tag_str();
            return (Some(SignalEvent::DeviceKeepalive { device_id: self.device_id_i64(), device_tag: Some(device_tag), timestamp: chrono::Utc::now() }),
                    Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)));
        }

        let device_tag = self.device_tag_str();

        if body.contains("<Response>") && body.contains("Catalog") {
            let channels = self.parse_catalog_response(body);
            tracing::debug!("[GB28181] parse_catalog_response returned {} channels", channels.len());
            for (i, ch) in channels.iter().enumerate() {
                tracing::debug!("[GB28181]   channel[{}]: id={}, name={}, parental={}", i, ch.device_id, ch.name, ch.parental);
            }
            if channels.is_empty() {
                tracing::info!("[GB28181] Catalog empty for device {} - IPC mode (device itself is the channel)", device_tag);
                self.pending_catalog_query = false;
                self.catalog_retry_count = 0;
                let device_channel = CatalogChannel {
                    device_id: device_tag.clone(),
                    name: "Camera 01".to_string(),
                    manufacturer: None,
                    model: None,
                    status: "ON".to_string(),
                    parental: false,
                    parent_id: Some(device_tag.clone()),
                    civil_code: None,
                    address: None,
                    ip_address: None,
                    port: None,
                    owner: None,
                    secrecy: None,
                    device_type: None,
                    ptz_type: None,
                    info: None,
                };
                return (
                    Some(SignalEvent::CatalogResponse {
                        device_id: self.device_id_i64(),
                        device_tag: Some(device_tag.clone()),
                        channels: vec![device_channel],
                    }),
                    Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)),
                );
            }
            self.pending_catalog_query = false;
            self.catalog_retry_count = 0;
            return (
                Some(SignalEvent::CatalogResponse {
                    device_id: self.device_id_i64(),
                    device_tag: Some(device_tag.clone()),
                    channels,
                }),
                Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)),
            );
        }

        if body.contains("Catalog") || body.contains("Query") {
            let channel_id = Self::parse_channel_id(body).unwrap_or_else(|| "00000000000000000000".to_string());
            let resp = format!("{}\r\n{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq).trim_end_matches("\r\n\r\n"),
                               self.build_catalog_response(&channel_id));
            return (Some(SignalEvent::QueryDeviceInfo { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()) }), Some(resp));
        }
        if body.contains("DeviceInfo") {
            let resp = format!("{}{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq),
                               self.build_device_info_response());
            return (Some(SignalEvent::QueryDeviceInfo { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()) }), Some(resp));
        }
        if body.contains("DeviceStatus") {
            let resp = format!("{}{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq),
                               self.build_device_status_response());
            return (Some(SignalEvent::QueryDeviceStatus { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()) }), Some(resp));
        }
        if body.contains("ConfigDownload") || body.contains("DeviceConfig") {
            let config_type = Self::parse_xml_field(body, "ConfigType").unwrap_or_else(|| "BasicParam".to_string());
            let resp = format!("{}{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq),
                               self.build_device_config_response(&config_type));
            tracing::info!("[GB28181] DeviceConfig query: device={} type={}", device_tag, config_type);
            return (Some(SignalEvent::QueryDeviceConfig { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()), config_type }), Some(resp));
        }
        if body.contains("ListPreset") || body.contains("PresetList") {
            let channel_id = Self::parse_channel_id(body).unwrap_or_else(|| device_tag.clone());
            let resp = format!("{}{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq),
                               self.build_preset_list_response(&channel_id));
            tracing::info!("[GB28181] PresetList query: device={} channel={}", device_tag, channel_id);
            return (Some(SignalEvent::PresetQuery { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()), channel_id }), Some(resp));
        }
        if body.contains("SetPreset") {
            let channel_id = Self::parse_channel_id(body).unwrap_or_else(|| device_tag.clone());
            let preset_id = Self::parse_xml_field(body, "PresetID").unwrap_or_else(|| "1".to_string());
            let preset_name = Self::parse_xml_field(body, "PresetName").unwrap_or_else(|| "Preset".to_string());
            let resp = format!("{}{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq),
                               self.build_preset_response(&channel_id, &preset_id, &preset_name, "SetPreset"));
            tracing::info!("[GB28181] SetPreset: device={} channel={} id={} name={}", device_tag, channel_id, preset_id, preset_name);
            return (Some(SignalEvent::PresetSet { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()), channel_id, preset_name }), Some(resp));
        }
        if body.contains("GotoPreset") {
            let channel_id = Self::parse_channel_id(body).unwrap_or_else(|| device_tag.clone());
            let preset_index = Self::parse_xml_field(body, "PresetIndex").and_then(|v| v.parse().ok()).unwrap_or(1);
            let resp = format!("{}{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq),
                               self.build_preset_response(&channel_id, &preset_index.to_string(), "", "GotoPreset"));
            tracing::info!("[GB28181] GotoPreset: device={} channel={} index={}", device_tag, channel_id, preset_index);
            return (Some(SignalEvent::PresetGoto { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()), channel_id, preset_index }), Some(resp));
        }
        if body.contains("RemovePreset") {
            let channel_id = Self::parse_channel_id(body).unwrap_or_else(|| device_tag.clone());
            let preset_index = Self::parse_xml_field(body, "PresetID").and_then(|v| v.parse().ok()).unwrap_or(1);
            let resp = format!("{}{}",
                               self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq),
                               self.build_preset_response(&channel_id, &preset_index.to_string(), "", "RemovePreset"));
            tracing::info!("[GB28181] RemovePreset: device={} channel={} index={}", device_tag, channel_id, preset_index);
            return (Some(SignalEvent::PresetRemove { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()), channel_id, preset_index }), Some(resp));
        }

        (Some(SignalEvent::DeviceKeepalive { device_id: self.device_id_i64(), device_tag: Some(device_tag.clone()), timestamp: chrono::Utc::now() }),
         Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)))
    }

    async fn handle_bye(
        &mut self, headers: &[(String, String)],
    ) -> (Option<SignalEvent>, Option<String>) {
        let via_branch = Self::parse_via_branch(headers);
        let cseq = Self::parse_cseq_line(headers);
        let call_id = Self::get_header(headers, "Call-ID").unwrap_or_default();

        if !self.is_registered {
            tracing::warn!("[GB28181] BYE: device not registered, rejecting");
            return (None, Some(self.build_403_response(&via_branch, &cseq)));
        }

        let sub_session = {
            let sessions = self.sessions.read().await;
            sessions.values().find(|s| s.call_id == call_id).cloned()
        };

        if let Some(session) = sub_session {
            if session.parent_device_id == session.channel_id {
                tracing::info!("[GB28181] Single device BYE: channel={}, stream={}", session.channel_id, session.stream_key);
                if let Some(app) = self.get_device_app(&session.channel_id).await {
                    let _ = self.deps.stream_manager.stop_stream(&app, &session.stream_key).await;
                }
                self.deps.rtp_tunnel.unregister(&session.stream_key);
                self.sessions.write().await.clear();
                return (
                    Some(SignalEvent::StopPlay {
                        device_id: self.device_id_i64(),
                        device_tag: Some(session.channel_id),
                        session_id: session.stream_key,
                    }),
                    Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)),
                );
            }
            tracing::info!("[GB28181] Sub-device BYE: channel={}, stream={}", session.channel_id, session.stream_key);
            if let Some(app) = self.get_device_app(&session.channel_id).await {
                let _ = self.deps.stream_manager.stop_stream(&app, &session.stream_key).await;
            }
            self.deps.rtp_tunnel.unregister(&session.stream_key);
            self.sessions.write().await.retain(|_, s| s.call_id != call_id);
            return (
                Some(SignalEvent::StopPlay {
                    device_id: self.device_id_i64(),
                    device_tag: Some(session.channel_id),
                    session_id: session.stream_key,
                }),
                Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)),
            );
        }

        tracing::warn!("[GB28181] BYE with unknown Call-ID: {}, iterating all sessions", call_id);
        for session in self.sessions.read().await.values() {
            if let Some(app) = self.get_device_app(&session.channel_id).await {
                let _ = self.deps.stream_manager.stop_stream(&app, &session.stream_key).await;
            }
            self.deps.rtp_tunnel.unregister(&session.stream_key);
        }
        self.sessions.write().await.clear();

        let device_tag = self.device_tag_str();
        let _ = self.deps.device_lookup.set_offline(&device_tag, Some("BYE received")).await;

        (Some(SignalEvent::StopPlay {
            device_id: self.device_id_i64(),
            device_tag: Some(device_tag),
            session_id: call_id,
        }), Some(self.build_sip_response_only_headers(200, "OK", &via_branch, &cseq)))
    }

    async fn handle_ack(&self, headers: &[(String, String)]) -> (Option<SignalEvent>, Option<String>) {
        let call_id = Self::get_header(headers, "Call-ID").unwrap_or_default();
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.values().find(|s| s.call_id == call_id) {
            tracing::info!("[GB28181] ACK for session: call_id={} channel={} stream={}", 
                call_id, session.channel_id, session.stream_key);
        } else {
            tracing::warn!("[GB28181] ACK for unknown session: {}, active_sessions={}", 
                call_id, sessions.len());
        }
        (None, None)
    }

    async fn handle_sip_response(
        &mut self, first_line: &str, headers: &[(String, String)], _body: &str,
    ) -> (Option<SignalEvent>, Option<String>) {
        let call_id = Self::get_header(headers, "Call-ID").unwrap_or_default();
        let cseq_line = Self::get_header(headers, "CSeq").unwrap_or_default();
        let is_invite_response = cseq_line.to_lowercase().contains("invite");

        let sub_session = {
            let sessions = self.sessions.read().await;
            sessions.values().find(|s| s.call_id == call_id).cloned()
        };

        let Some(session) = sub_session else {
            tracing::debug!("[GB28181] Response for unknown session: {} status={}", call_id, first_line);
            return (None, None);
        };

        tracing::info!("[GB28181] Response for sub-session: channel={} call_id={} status={}", session.channel_id, call_id, first_line);

        if !is_invite_response {
            return (None, None);
        }

        let is_200 = first_line.contains(" 200 ");
        let is_error = first_line.starts_with("SIP/2.0 ") && (
            first_line.contains(" 401 ") || first_line.contains(" 403 ") ||
            first_line.contains(" 404 ") || first_line.contains(" 488 ") ||
            first_line.contains(" 500 ") || first_line.contains(" 503 ") ||
            first_line.contains(" 600 "));

        if is_error {
            tracing::warn!("[GB28181] INVITE failed: channel={} status={}", session.channel_id, first_line);
            self.sessions.write().await.retain(|_, s| s.call_id != call_id);
            self.deps.rtp_tunnel.unregister(&session.stream_key);
            return (None, None);
        }

        if !is_200 {
            return (None, None);
        }

        let via_received = Self::parse_via_received(headers);
        let via_rport = Self::parse_via_rport(headers);

        if let (Some(received_ip), Some(rport)) = (&via_received, via_rport) {
            tracing::debug!("[GB28181] NAT detected: device at {}:{}", received_ip, rport);
            self.sessions.write().await
                .entry(session.channel_id.clone())
                .and_modify(|s| {
                    s.device_nated_ip = Some(received_ip.clone());
                    s.device_nated_port = Some(rport);
                });
        }

        let to_tag = Self::get_header(headers, "To")
            .and_then(|v| Self::parse_tag_from_header(&v));

        if let Some(expected_tag) = &session.to_tag {
            if let Some(received_tag) = &to_tag {
                if received_tag != expected_tag {
                    tracing::warn!("[GB28181] To-tag mismatch! expected={} received={}, Call-ID={}", expected_tag, received_tag, session.call_id);
                }
            }
        }

        let to_tag_to_use = to_tag.clone().unwrap_or_default();
        if session.to_tag.is_none() && to_tag.is_some() {
            self.sessions.write().await
                .entry(session.channel_id.clone())
                .and_modify(|s| s.to_tag = to_tag.clone());
            tracing::debug!("[GB28181] Stored device to_tag={} for channel={}", to_tag.as_deref().unwrap_or(""), session.channel_id);
        }

        let contact = headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Contact"))
            .and_then(|(_, v)| {
                let v = v.trim();
                v.strip_prefix('<').and_then(|s| s.strip_suffix('>'))
                    .or_else(|| v.strip_prefix("sip:").filter(|s| !s.starts_with('<')))
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| format!("sip:{}@{}:{}", session.parent_device_id, session.device_host, session.device_port));

        let (ack_target_host, ack_target_port) = if let (Some(nated_ip), Some(nated_port)) = (&session.device_nated_ip, session.device_nated_port) {
            (nated_ip.clone(), nated_port)
        } else {
            let target_ip = via_received.unwrap_or_else(|| session.device_host.clone());
            let target_port = match via_rport {
                Some(0) => {
                    if session.transport == TransportProtocol::Tcp {
                        session.device_port
                    } else {
                        self.remote_addr.map(|a| a.port()).unwrap_or(session.device_port)
                    }
                }
                Some(port) if port > 0 => port,
                _ => session.device_port,
            };
            (target_ip, target_port)
        };

        let via = if session.transport == TransportProtocol::Tcp {
            format!("SIP/2.0/TCP {}:{};branch={};rport", session.server_ip, session.server_port, session.via_branch)
        } else {
            format!("SIP/2.0/UDP {}:{};branch={};rport", session.server_ip, session.server_port, session.via_branch)
        };

        let ack_target = format!("sip:{}@{}:{}", session.parent_device_id, ack_target_host, ack_target_port);
        let to_uri_with_tag = if to_tag_to_use.is_empty() {
            session.to_uri.clone()
        } else {
            format!("{};tag={}", session.to_uri, to_tag_to_use)
        };
        let ack = format!(
            "ACK {ack_target} SIP/2.0\r\n\
             Via: {via}\r\n\
             From: <{from_uri}>;tag={from_tag}\r\n\
             To: <{to_uri_with_tag}>\r\n\
             Call-ID: {call_id}\r\n\
             CSeq: {seq} ACK\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             \r\n",
            ack_target = ack_target,
            via = via,
            from_uri = session.from_uri,
            from_tag = session.from_tag,
            to_uri_with_tag = to_uri_with_tag,
            call_id = session.call_id,
            seq = session.seq
        );
        tracing::debug!("[GB28181] Sending ACK to {} (via NAT: {}:{})", ack_target, ack_target_host, ack_target_port);
        self.send(ack.as_bytes()).await.ok();

        let channel_tag = &session.channel_id;
        let device_tag = self.device_tag_str();
        let start_result = self.deps.stream_manager
            .start_gb28181_stream(&device_tag, channel_tag, &session.stream_key)
            .await;

        if start_result.is_ok() {
            self.sessions.write().await
                .entry(session.channel_id.clone())
                .and_modify(|s| s.media_started = true);

            if let Some(app) = self.get_device_app(&session.channel_id).await {
                if let Some(stream) = self.deps.stream_manager.get_stream_by_stream_key(&app, &session.stream_key).await {
                    let mut s = stream.clone();
                    s.start();
                    let _ = self.deps.stream_manager.update_stream_state(&s).await;
                }
            }

            tracing::info!("[GB28181] Stream started: channel={} stream={}", session.channel_id, session.stream_key);
        } else {
            tracing::error!("[GB28181] Failed to start stream: channel={} err={:?}", session.channel_id, start_result);
        }

        (Some(SignalEvent::StartPlay {
            device_id: self.device_id_i64(),
            device_tag: Some(session.channel_id.clone()),
            session_id: session.stream_key.clone(),
            channel_id: Some(session.channel_id.clone()),
            transport: TransportType::UDP,
            media_server_name: Some(session.media_server_name.clone()),
        }), None)
    }

    fn handle_options(&mut self, headers: &[(String, String)]) -> Option<String> {
        Some(self.build_sip_response_only_headers(200, "OK", &Self::parse_via_branch(headers), &Self::parse_cseq_line(headers)))
    }

    async fn handle_subscribe(&mut self, headers: &[(String, String)], body: &str) -> (Option<SignalEvent>, Option<String>) {
        let via_branch = Self::parse_via_branch(headers);
        let cseq = Self::parse_cseq_line(headers);

        if !self.is_registered {
            tracing::warn!("[GB28181] SUBSCRIBE: device not registered, rejecting");
            return (None, Some(self.build_403_response(&via_branch, &cseq)));
        }

        let event_type = if body.contains("Event") {
            Self::parse_xml_field(body, "Event").unwrap_or_else(|| "presence".to_string())
        } else {
            "presence".to_string()
        };

        let expires = Self::get_header(headers, "Expires")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3600);

        let device_tag = self.device_tag_str();
        let device_id = self.device_id_i64();

        let sub_key = format!("{}:{}", device_id, event_type);
        let subscription = Subscription {
            event_type: event_type.clone(),
            expires,
            created_at: chrono::Utc::now(),
        };
        self.subscriptions.write().await.insert(sub_key, subscription);

        tracing::info!("[GB28181] Device {} subscribed to event={} expires={}s", device_tag, event_type, expires);

        let resp = format!(
            "SIP/2.0 200 OK\r\n\
             Via: {}\r\n\
             From: <sip:{}>\r\n\
             To: <sip:{}>;tag={}\r\n\
             Call-ID: {}\r\n\
             CSeq: {} SUBSCRIBE\r\n\
             Expires: {}\r\n\
             Contact: <sip:{}>\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Content-Length: 0\r\n\r\n",
            self.via_header(&via_branch),
            device_tag,
            self.deps.config.signaling_server.server_gb_id,
            self.to_tag.as_deref().unwrap_or(""),
            Self::get_header(headers, "Call-ID").unwrap_or_default(),
            cseq,
            expires,
            device_tag
        );

        (Some(SignalEvent::DeviceSubscribe {
            device_id,
            device_tag: Some(device_tag),
            event_types: vec![event_type],
            expires,
        }), Some(resp))
    }

    pub async fn cleanup_expired_subscriptions(&self) -> usize {
        let mut count = 0;
        let now = chrono::Utc::now();
        let mut subs = self.subscriptions.write().await;
        let keys_to_remove: Vec<String> = subs.iter()
            .filter(|(_, sub)| {
                let age = now.signed_duration_since(sub.created_at);
                age.num_seconds() as u32 >= sub.expires
            })
            .map(|(k, _)| k.clone())
            .collect();
        for key in keys_to_remove {
            subs.remove(&key);
            count += 1;
        }
        if count > 0 {
            tracing::info!("[GB28181] Cleaned up {} expired subscriptions", count);
        }
        count
    }

    fn device_tag_str(&self) -> String {
        self.device_tag.clone().unwrap_or_default()
    }

    fn device_id_i64(&self) -> i64 {
        self.device_id.unwrap_or(0)
    }

    fn device_id_str(&self) -> Option<String> {
        self.device_id.map(|i| i.to_string())
    }

    fn has_timed_out(&self) -> bool {
        if let Some(timeout_secs) = self.idle_timeout() {
            if let Some(last_msg) = self.last_message_at {
                let elapsed = chrono::Utc::now().signed_duration_since(last_msg);
                return elapsed.num_seconds() as u64 > timeout_secs;
            }
        }
        false
    }

}

// #[async_trait]
// impl EventHandler for Gb28181Adapter {
//     async fn handle(&mut self, event: SignalEvent) {
//         let SignalEvent::PtzControl { device_id: did, command, speed } = event else { return; };
// 
//         let dev_tag = self.device_tag_str();
//         let dev_id_str = self.device_id_str();
//         if Some(did.clone()) != dev_id_str && did != dev_tag {
//             return;
//         }
// 
//         let spd = speed.unwrap_or(50);
//         let xml = match command {
//             PtzCommand::Up => Self::build_ptz_xml(&dev_tag, "up", spd),
//             PtzCommand::Down => Self::build_ptz_xml(&dev_tag, "down", spd),
//             PtzCommand::Left => Self::build_ptz_xml(&dev_tag, "left", spd),
//             PtzCommand::Right => Self::build_ptz_xml(&dev_tag, "right", spd),
//             PtzCommand::ZoomIn => Self::build_ptz_xml(&dev_tag, "zoom_in", spd),
//             PtzCommand::ZoomOut => Self::build_ptz_xml(&dev_tag, "zoom_out", spd),
//             PtzCommand::Stop => Self::build_ptz_xml(&dev_tag, "stop", 0),
//             PtzCommand::ContinuousMove { pan, tilt, zoom } => {
//                 if pan.abs() < 0.01 && tilt.abs() < 0.01 && zoom.abs() < 0.01 {
//                     Self::build_ptz_xml(&dev_tag, "stop", 0)
//                 } else {
//                     let dir = Self::best_direction(pan, tilt, zoom);
//                     let spd = ((pan.abs().max(tilt.abs()).max(zoom.abs()) * 100.0) as u8).clamp(1, 100);
//                     Self::build_ptz_xml(&dev_tag, dir, spd)
//                 }
//             }
//             _ => return,
//         };
// 
//         if let Some(ref write_arc) = self.write {
//             let mut write = write_arc.write().await;
//             let _ = write.write_all(xml.as_bytes()).await;
//             let _ = write.flush().await;
//         } else if let Some(ref addr) = self.udp_peer {
//             if let Some(sender) = super::get_udp_sender() {
//                 let _ = sender.send_to(xml.as_bytes(), *addr).await;
//             }
//         }
//     }
// }



#[async_trait]
impl SignalAdapter for Gb28181Adapter {
    async fn parse(&mut self, data: &[u8]) -> Result<Vec<SignalEvent>> {
        let mut buf = (*self.recv_buffer).clone();
        buf.extend_from_slice(data);
        self.recv_buffer = Arc::new(buf);

        const MAX_BUFFER_SIZE: usize = 1024 * 1024;
        if self.recv_buffer.len() > MAX_BUFFER_SIZE {
            tracing::error!("[GB28181] Buffer overflow, clearing. Device: {:?}", self.device_tag);
            self.recv_buffer = Arc::new(Vec::new());
            return Ok(Vec::new());
        }

        let mut events = Vec::new();

        loop {
            let buffer = (*self.recv_buffer).clone();
            
            let (sip_msg, consumed) = match SipMessage::from_buffer(&buffer) {
                Some((msg, len)) => (msg, len),
                None => break,
            };

            self.last_message_at = Some(chrono::Utc::now());
            self.recv_buffer = Arc::new(buffer[consumed..].to_vec());

            let (first_line, headers, body) = sip_msg.to_old_format();

            let (event, response) = if first_line.starts_with("SIP/2.0") {
                self.handle_sip_response(&first_line, &headers, &body).await
            } else if first_line.starts_with("REGISTER") {
                self.handle_register(&headers, &body).await
            } else if first_line.starts_with("MESSAGE") {
                self.handle_message(&headers, &body).await
            } else if first_line.starts_with("BYE") {
                self.handle_bye(&headers).await
            } else if first_line.starts_with("ACK") {
                self.handle_ack(&headers).await
            } else if first_line.starts_with("OPTIONS") {
                let resp = self.handle_options(&headers);
                (None, resp)
            } else if first_line.starts_with("SUBSCRIBE") {
                self.handle_subscribe(&headers, &body).await
            } else {
                (None, None)
            };

            if let Some(resp) = &response {
                tracing::debug!("[GB28181] Sending response to {}: {}", self.udp_peer.map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string()), &resp[..resp.len().min(200)]);
                if let Err(e) = self.send(resp.as_bytes()).await {
                    tracing::error!("[GB28181] Failed to send response: {}", e);
                }
            }

            if self.pending_catalog_query {
                self.pending_catalog_query = false;
                let query = self.build_catalog_query();
                let query_str = String::from_utf8_lossy(&query);
                tracing::info!("[GB28181] Sending catalog query to {}: {}", self.device_tag_str(), query_str);
                let _ = self.send(&query).await;
            }

            if let Some(evt) = event {
                events.push(evt);
            }
        }
        Ok(events)
    }

    async fn on_connected(&mut self, addr: SocketAddr) -> Result<()> {
        self.remote_addr = Some(addr);
        Ok(())
    }

    async fn on_disconnected(&mut self) -> Result<()> {
        let device_tag = self.device_tag_str();
        if !device_tag.is_empty() {
            tracing::info!("[GB28181] Device {} disconnected, cleaning {} sessions", device_tag, self.sessions.read().await.len());
            
            let sessions: Vec<SubSession> = self.sessions.read().await.values().cloned().collect();
            for session in &sessions {
                if let Err(e) = self.send_bye_to_session(session).await {
                    tracing::warn!("[GB28181] Failed to send BYE to {}:{}", session.device_host, e);
                }
            }

            if let Some(ref name) = self.event_handler_name {
                registry().infra.event_bus.off(name);
            }
            let _ = self.deps.device_lookup.set_offline(&device_tag, Some("Connection closed")).await;

            if let Err(e) = self.deps.stream_manager.stop_streams_by_device(&device_tag).await {
                tracing::warn!("[GB28181] on_disconnected: failed to stop streams for device {}: {}", device_tag, e);
            }

            for session in sessions {
                self.deps.rtp_tunnel.unregister(&session.stream_key);
            }
            self.sessions.write().await.clear();
        }
        if let Some(ref dt) = self.device_tag {
            (self.unregister_fn)(dt.clone());
        }
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref write_arc) = self.write {
            let mut write = write_arc.write().await;
            write.write_all(data).await
                .map_err(|e| AppError::Internal(format!("TCP write: {}", e)))?;
            let _ = write.flush().await;
            return Ok(());
        }
        if let Some(ref addr) = self.udp_peer {
            if let Some(sender) = super::get_udp_sender() {
                if let Err(e) = sender.send_to(data, *addr).await {
                    tracing::error!("[GB28181] UDP send failed to {}: {}", addr, e);
                    return Err(AppError::Internal(format!("UDP send: {}", e)));
                }
            } else {
                tracing::warn!("[GB28181] UDP sender not initialized");
            }
        } else {
            tracing::warn!("[GB28181] UDP peer not set, cannot send response");
        }
        Ok(())
    }

    fn protocol_type(&self) -> ProtocolType { ProtocolType::Gb28181 }
    fn name(&self) -> &'static str { "GB28181" }
    fn keepalive(&self) -> bool { true }
    fn idle_timeout(&self) -> Option<u64> { Some(300) }

    fn set_tcp_write(&mut self, write: OwnedWriteHalf) {
        self.write = Some(Arc::new(tokio::sync::RwLock::new(write)));
    }

    async fn set_udp_peer(&mut self, addr: SocketAddr) -> Result<()> {
        self.udp_peer = Some(addr);
        self.transport = TransportProtocol::Udp;
        Ok(())
    }

    async fn start(&mut self, device_tag: &str) -> Result<()> {
        let device = self.deps.device_lookup.find_by_tag(device_tag).await
            .ok_or_else(|| AppError::NotFound(format!("Device not found: {}", device_tag)))?;

        if let Some(ext) = &device.extended {
            if let Some(transport) = ext.get("gb_transport").and_then(|v| v.as_str()) {
                let transport_str = match transport.to_uppercase().as_str() {
                    "TCP" => {
                        self.transport = TransportProtocol::Tcp;
                        "TCP"
                    },
                    _ => {
                        self.transport = TransportProtocol::Udp;
                        "UDP"
                    },
                };
                tracing::info!("[GB28181] Using transport {} for device {}", transport_str, device_tag);
            }
        }

        let stream_key = format!("gb_{}", device_tag);
        let (rtp_port, server_ip, media_server_name) = self.alloc_rtp_port(&stream_key, device.media_server_tag.as_deref()).await?;

        let sn = self.seq.fetch_add(1, Ordering::SeqCst);
        let ssrc = format!("{:08x}", rand::random::<u32>());
        let call_id = uuid::Uuid::new_v4().to_string().replace("-", "");
        let from_tag = uuid::Uuid::new_v4().to_string().replace("-", "");
        let branch = format!("z9hG4bK{}", &call_id[..7]);
        let server_port = self.deps.config.server.port;
        let server_gb_id = &self.deps.config.signaling_server.server_gb_id;
        let server_gb_domain = &self.deps.config.signaling_server.server_gb_domain;

        let parent_device_id = device.parent_device_tag.clone()
            .unwrap_or_else(|| device_tag.to_string());
        let channel_id = device_tag.to_string();
        let device_host = device.host.clone();
        let device_port = device.port;

        let via = if self.transport == TransportProtocol::Tcp {
            format!("SIP/2.0/TCP {server_ip}:{server_port};branch={branch};rport")
        } else {
            format!("SIP/2.0/UDP {server_ip}:{server_port};branch={branch};rport")
        };

        let sdp = self.build_sdp(&server_gb_id, &server_ip, rtp_port, &ssrc, device.stream_config.as_ref(), None, None);
        let sdp_len = sdp.len();
        let to_uri = format!("sip:{}@{}:{}", parent_device_id, device_host, device_port);
        let from_uri = format!("sip:{}@{}:{}", server_gb_id, server_ip, server_port);
        let subject = format!(
            "{}:{}:0",
            Self::truncate_20(&channel_id),
            Self::truncate_20(server_gb_id)
        );

        let session = SubSession {
            call_id: call_id.clone(),
            seq: sn,
            channel_id: channel_id.clone(),
            parent_device_id: parent_device_id.clone(),
            stream_key: stream_key.clone(),
            media_server_name: media_server_name.clone(),
            from_uri: from_uri.clone(),
            to_uri: to_uri.clone(),
            from_tag: from_tag.clone(),
            to_tag: None,
            via_branch: branch.clone(),
            server_ip: server_ip.clone(),
            server_port,
            transport: self.transport,
            device_host: device_host.clone(),
            device_port,
            rtp_port,
            media_started: false,
            device_nated_ip: None,
            device_nated_port: None,
        };

        let old_session = self.sessions.write().await.insert(device_tag.to_string(), session.clone());
        if let Some(old) = old_session {
            tracing::info!("[GB28181] start: cleaning up old session for channel={}", device_tag);
            if old.media_started {
                if let Some(app) = self.get_device_app(&old.channel_id).await {
                    let _ = self.deps.stream_manager.stop_stream(&app, &old.stream_key).await;
                }
                self.deps.rtp_tunnel.unregister(&old.stream_key);
            }
        }

        let invite = format!(
            "INVITE sip:{to_uri} SIP/2.0\r\n\
             Via: {via}\r\n\
             From: <sip:{from_uri}>;domain={server_gb_domain};tag={from_tag}\r\n\
             To: <sip:{to_uri}>\r\n\
             Call-ID: {call_id}\r\n\
             CSeq: {sn} INVITE\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Subject: {subject}\r\n\
             Content-Type: application/sdp\r\n\
             Content-Length: {sdp_len}\r\n\
             \r\n\
             {sdp}",
            to_uri = to_uri,
            via = via,
            from_uri = from_uri,
            server_gb_domain = server_gb_domain,
            from_tag = from_tag,
            call_id = call_id,
            sn = sn,
            subject = subject,
            sdp_len = sdp_len,
            sdp = sdp
        );

        tracing::info!("[GB28181] Sending INVITE to device={}, stream={}, rtp={}", device_tag, stream_key, rtp_port);
        self.send(invite.as_bytes()).await?;

        Ok(())
    }

    async fn start_playback(&mut self, device_tag: &str, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<()> {
        let device = self.deps.device_lookup.find_by_tag(device_tag).await
            .ok_or_else(|| AppError::NotFound(format!("Device not found: {}", device_tag)))?;

        let stream_key = format!("gb_{}", device_tag);
        let (rtp_port, server_ip, media_server_name) = self.alloc_rtp_port(&stream_key, device.media_server_tag.as_deref()).await?;

        let sn = self.seq.fetch_add(1, Ordering::SeqCst);
        let ssrc = format!("{:08x}", rand::random::<u32>());
        let call_id = uuid::Uuid::new_v4().to_string().replace("-", "");
        let from_tag = uuid::Uuid::new_v4().to_string().replace("-", "");
        let branch = format!("z9hG4bK{}", &call_id[..7]);
        let server_port = self.deps.config.server.port;
        let server_gb_id = &self.deps.config.signaling_server.server_gb_id;
        let server_gb_domain = &self.deps.config.signaling_server.server_gb_domain;

        let parent_device_id = device.parent_device_tag.clone()
            .unwrap_or_else(|| device_tag.to_string());
        let channel_id = device_tag.to_string();
        let device_host = device.host.clone();
        let device_port = device.port;

        let via = if self.transport == TransportProtocol::Tcp {
            format!("SIP/2.0/TCP {server_ip}:{server_port};branch={branch};rport")
        } else {
            format!("SIP/2.0/UDP {server_ip}:{server_port};branch={branch};rport")
        };

        let sdp = self.build_sdp(&server_gb_id, &server_ip, rtp_port, &ssrc, device.stream_config.as_ref(), Some(start_time), Some(end_time));
        let sdp_len = sdp.len();
        let to_uri = format!("sip:{}@{}:{}", parent_device_id, device_host, device_port);
        let from_uri = format!("sip:{}@{}:{}", server_gb_id, server_ip, server_port);
        let start_ntp = Self::datetime_to_ntp_string(start_time);
        let end_ntp = Self::datetime_to_ntp_string(end_time);
        let subject = format!(
            "{}:{}:{},{}",
            Self::truncate_20(&channel_id),
            Self::truncate_20(server_gb_id),
            start_ntp,
            end_ntp
        );

        let session = SubSession {
            call_id: call_id.clone(),
            seq: sn,
            channel_id: channel_id.clone(),
            parent_device_id: parent_device_id.clone(),
            stream_key: stream_key.clone(),
            media_server_name: media_server_name.clone(),
            from_uri: from_uri.clone(),
            to_uri: to_uri.clone(),
            from_tag: from_tag.clone(),
            to_tag: None,
            via_branch: branch.clone(),
            server_ip: server_ip.clone(),
            server_port,
            transport: self.transport,
            device_host: device_host.clone(),
            device_port,
            rtp_port,
            media_started: false,
            device_nated_ip: None,
            device_nated_port: None,
        };

        let old_session = self.sessions.write().await.insert(device_tag.to_string(), session.clone());
        if let Some(old) = old_session {
            tracing::info!("[GB28181] start_playback: cleaning up old session for channel={}", device_tag);
            if old.media_started {
                if let Some(app) = self.get_device_app(&old.channel_id).await {
                    let _ = self.deps.stream_manager.stop_stream(&app, &old.stream_key).await;
                }
                self.deps.rtp_tunnel.unregister(&old.stream_key);
            }
        }

        let invite = format!(
            "INVITE sip:{to_uri} SIP/2.0\r\n\
             Via: {via}\r\n\
             From: <sip:{from_uri}>;domain={server_gb_domain};tag={from_tag}\r\n\
             To: <sip:{to_uri}>\r\n\
             Call-ID: {call_id}\r\n\
             CSeq: {sn} INVITE\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Subject: {subject}\r\n\
             Content-Type: application/sdp\r\n\
             Content-Length: {sdp_len}\r\n\
             \r\n\
             {sdp}",
            to_uri = to_uri,
            via = via,
            from_uri = from_uri,
            server_gb_domain = server_gb_domain,
            from_tag = from_tag,
            call_id = call_id,
            sn = sn,
            subject = subject,
            sdp_len = sdp_len,
            sdp = sdp
        );

        tracing::info!("[GB28181] Sending Playback INVITE to device={}, stream={}, rtp={}, start={}, end={}", device_tag, stream_key, rtp_port, start_ntp, end_ntp);
        self.send(invite.as_bytes()).await?;

        Ok(())
    }

    async fn ptz_control(&mut self, channel_id: &str, command: &crate::protocol::event::PtzCommand, speed: Option<u8>) -> Result<()> {
        let device = self.deps.device_lookup.find_by_tag(channel_id).await
            .ok_or_else(|| AppError::NotFound(format!("Device not found: {}", channel_id)))?;

        let parent_device_id = device.parent_device_tag.clone().unwrap_or_else(|| channel_id.to_string());
        let spd = speed.unwrap_or(50);

        match command {
            PtzCommand::ContinuousMove { pan, tilt, zoom } => {
                if pan.abs() < 0.01 && tilt.abs() < 0.01 && zoom.abs() < 0.01 {
                    let ptz_body = Self::build_ptz_xml(channel_id, "stop", 0);
                    let msg = self.build_ptz_message(channel_id, &parent_device_id, &ptz_body);
                    tracing::debug!("[GB28181] PTZ: channel={} compound=stop", channel_id);
                    self.send(msg.as_bytes()).await?;
                } else {
                    let ptz_body = Self::build_ptz_xml_compound(channel_id, *pan, *tilt, *zoom, spd);
                    let msg = self.build_ptz_message(channel_id, &parent_device_id, &ptz_body);
                    tracing::debug!("[GB28181] PTZ: channel={} compound=pan:{:.2} tilt:{:.2} zoom:{:.2}", channel_id, pan, tilt, zoom);
                    self.send(msg.as_bytes()).await?;
                }
            }
            PtzCommand::Stop => {
                let ptz_body = Self::build_ptz_xml(channel_id, "stop", 0);
                let msg = self.build_ptz_message(channel_id, &parent_device_id, &ptz_body);
                tracing::debug!("[GB28181] PTZ: channel={} dir=stop", channel_id);
                self.send(msg.as_bytes()).await?;
            }
            _ => {
                let (direction, spd_val) = match command {
                    PtzCommand::Up => ("up", spd),
                    PtzCommand::Down => ("down", spd),
                    PtzCommand::Left => ("left", spd),
                    PtzCommand::Right => ("right", spd),
                    PtzCommand::ZoomIn => ("zoom_in", spd),
                    PtzCommand::ZoomOut => ("zoom_out", spd),
                    PtzCommand::FocusIn => ("focus_in", spd),
                    PtzCommand::FocusOut => ("focus_out", spd),
                    _ => return Ok(()),
                };

                let ptz_body = Self::build_ptz_xml(channel_id, direction, spd_val);
                let msg = self.build_ptz_message(channel_id, &parent_device_id, &ptz_body);
                tracing::debug!("[GB28181] PTZ: channel={} dir={} spd={}", channel_id, direction, spd_val);
                self.send(msg.as_bytes()).await?;
            }
        }
        Ok(())
    }

    async fn send_notify(&mut self, device_tag: &str, event_type: &str, content: &str) -> Result<()> {
        let device = self.deps.device_lookup.find_by_tag(device_tag).await
            .ok_or_else(|| AppError::NotFound(format!("Device not found: {}", device_tag)))?;

        let sn = self.seq.fetch_add(1, Ordering::SeqCst);
        let call_id = uuid::Uuid::new_v4().to_string().replace("-", "");
        let from_tag = uuid::Uuid::new_v4().to_string().replace("-", "");
        let to_tag = uuid::Uuid::new_v4().to_string().replace("-", "");
        let branch = format!("z9hG4bK{}", &call_id[..7]);
        let server_port = self.deps.config.server.port;
        let server_ip = self.remote_addr.map(|a| a.ip().to_string()).unwrap_or_else(|| "0.0.0.0".to_string());
        let server_gb_id = &self.deps.config.signaling_server.server_gb_id;
        let server_gb_domain = &self.deps.config.signaling_server.server_gb_domain;

        let parent_device_id = device.parent_device_tag.clone().unwrap_or_else(|| device_tag.to_string());
        let device_host = device.host.clone();
        let device_port = device.port;

        let via = if self.transport == TransportProtocol::Tcp {
            format!("SIP/2.0/TCP {server_ip}:{server_port};branch={branch};rport")
        } else {
            format!("SIP/2.0/UDP {server_ip}:{server_port};branch={branch};rport")
        };

        let from_uri = format!("sip:{}@{}:{}", server_gb_id, server_ip, server_port);
        let to_uri = format!("sip:{}@{}:{}", parent_device_id, device_host, device_port);

        let notify = format!(
            "NOTIFY sip:{to_uri} SIP/2.0\r\n\
             Via: {via}\r\n\
             From: <sip:{from_uri}>;domain={server_gb_domain};tag={from_tag}\r\n\
             To: <sip:{to_uri}>;tag={to_tag}\r\n\
             Call-ID: {call_id}\r\n\
             CSeq: {sn} NOTIFY\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Event: {event_type}\r\n\
             Content-Type: Application/MANSCDP+xml\r\n\
             Content-Length: {content_len}\r\n\
             \r\n\
             {content}",
            to_uri = to_uri,
            via = via,
            from_uri = from_uri,
            server_gb_domain = server_gb_domain,
            from_tag = from_tag,
            to_tag = to_tag,
            call_id = call_id,
            sn = sn,
            event_type = event_type,
            content_len = content.len(),
            content = content
        );

        tracing::info!("[GB28181] Sending NOTIFY to device={} event={}", device_tag, event_type);
        self.send(notify.as_bytes()).await?;
        Ok(())
    }
}

impl Gb28181Adapter {
    pub async fn notify_alarm(&mut self, device_tag: &str, alarm_type: &str, description: &str) -> Result<()> {
        let sn = chrono::Utc::now().timestamp_millis() % 1000000;
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>Alarm</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<AlarmType>{}</AlarmType><AlarmPriority>0</AlarmPriority><AlarmMethod>1</AlarmMethod>
<AlarmTime>{}</AlarmTime><Description>{}</Description></Notify>"#,
            sn,
            device_tag,
            alarm_type,
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S"),
            description
        );
        self.send_notify(device_tag, "Alarm", &content).await
    }

    pub async fn notify_device_status(&mut self, device_tag: &str, online: bool) -> Result<()> {
        let sn = chrono::Utc::now().timestamp_millis() % 1000000;
        let status = if online { "ON" } else { "OFF" };
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Notify><CmdType>DeviceStatus</CmdType><SN>{}</SN><DeviceID>{}</DeviceID>
<Online>{}</Online><Status>OK</Status></Notify>"#,
            sn,
            device_tag,
            status
        );
        self.send_notify(device_tag, "presence", &content).await
    }

    pub fn build_audio_invite_sdp(&self, device_ip: &str, audio_port: u16) -> String {
        let (platform_id, platform_ip, platform_port) = crate::protocol::gb28181::get_gb28181_platform_config()
            .get()
            .map(|(id, _, ip, port)| (id.as_str(), ip.as_str(), *port))
            .unwrap_or(("34020000000000000000", "0.0.0.0", 5060));
        
        format!(
            r#"v=0
o={} 0 0 IN IP4 {}
s=Talk
c=IN IP4 {}
t=0 0
m=audio {} RTP/AVP 8
a=sendonly
a=rtpmap:8 PCMA/8000
"#,
            platform_id, platform_ip, platform_ip, audio_port
        )
    }

    pub fn build_audio_invite_sdp_tcp(&self, device_ip: &str, tcp_port: u16) -> String {
        let (platform_id, platform_ip, platform_port) = crate::protocol::gb28181::get_gb28181_platform_config()
            .get()
            .map(|(id, _, ip, port)| (id.as_str(), ip.as_str(), *port))
            .unwrap_or(("34020000000000000000", "0.0.0.0", 5060));
        
        format!(
            r#"v=0
o={} 0 0 IN IP4 {}
s=Talk
c=IN IP4 {}
t=0 0
m=audio {} TCP/RTP/AVP 8
a=setup:passive
a=connection:new
a=rtpmap:8 PCMA/8000
"#,
            platform_id, platform_ip, platform_ip, tcp_port
        )
    }

    pub async fn start_audio_talk(
        &self,
        device_tag: &str,
        device_ip: &str,
        device_audio_port: u16,
        platform_audio_port: u16,
        transport: TransportType,
    ) -> Result<()> {
        let call_id = format!("audio-{}", chrono::Utc::now().timestamp_millis());
        let from_tag = format!("ft-{}", chrono::Utc::now().timestamp_millis() % 1000000);
        
        let sdp = match transport {
            TransportType::TCP => self.build_audio_invite_sdp_tcp(device_ip, platform_audio_port),
            _ => self.build_audio_invite_sdp(device_ip, device_audio_port),
        };
        
        let tcp_stream = if transport == TransportType::TCP {
            let addr = format!("{}:{}", device_ip, device_audio_port);
            match tokio::net::TcpStream::connect(&addr).await {
                Ok(stream) => {
                    tracing::info!("[GB28181] TCP audio connection established to {}", addr);
                    Some(Arc::new(tokio::sync::RwLock::new(stream)))
                }
                Err(e) => {
                    tracing::warn!("[GB28181] Failed to establish TCP audio connection: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        let session = AudioTalkSession {
            device_tag: device_tag.to_string(),
            device_ip: device_ip.to_string(),
            device_audio_port,
            platform_audio_port,
            call_id: call_id.clone(),
            from_tag: from_tag.clone(),
            to_tag: None,
            start_time: chrono::Utc::now(),
            audio_socket: None,
            tcp_stream,
            rtp_ssrc: rand_ssrc(),
            rtp_sequence: rand_u16(),
            rtp_timestamp: rand_u32(),
        };
        
        let mut sessions = self.audio_talk_sessions.write().await;
        sessions.insert(device_tag.to_string(), session);
        
        tracing::info!("[GB28181] Starting audio talk: device={} ip={} port={} transport={:?}", device_tag, device_ip, device_audio_port, transport);
        
        let (platform_id, platform_ip, platform_port) = crate::protocol::gb28181::get_gb28181_platform_config()
            .get()
            .map(|(id, _, ip, port)| (id.as_str(), ip.as_str(), *port))
            .unwrap_or(("34020000000000000000", "0.0.0.0", 5060));
        
        let via_transport = match transport {
            TransportType::TCP => "TCP",
            _ => "UDP",
        };
        
        let sip_msg = format!(
            "INVITE sip:{}@{}:{} SIP/2.0\r\n\
             Via: SIP/2.0/{} {}:{};branch=z9hG4bK{};rport\r\n\
             From: <sip:{}@{}:{}>;tag={}\r\n\
             To: <sip:{}@{}:{}>\r\n\
             Call-ID: {}\r\n\
             CSeq: 1 INVITE\r\n\
             Contact: <sip:{}@{}:{}>\r\n\
             Content-Type: application/sdp\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Content-Length: {}\r\n\r\n\
             {}",
            device_tag, device_ip, device_audio_port,
            via_transport, platform_ip, platform_port,
            chrono::Utc::now().timestamp_millis(),
            platform_id, platform_ip, platform_port, &from_tag,
            device_tag, device_ip, device_audio_port,
            &call_id,
            platform_id, platform_ip, platform_port,
            sdp.len(),
            sdp
        );
        
        if let Some(sender) = crate::protocol::gb28181::get_udp_sender() {
            sender.send(sip_msg.as_bytes()).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        
        Ok(())
    }

    pub async fn stop_audio_talk(&self, device_tag: &str) -> Result<()> {
        let mut sessions = self.audio_talk_sessions.write().await;
        if let Some(session) = sessions.remove(device_tag) {
            tracing::info!("[GB28181] Stopping audio talk: device={}", device_tag);
            
            let (platform_id, platform_ip, platform_port) = crate::protocol::gb28181::get_gb28181_platform_config()
                .get()
                .map(|(id, _, ip, port)| (id.as_str(), ip.as_str(), *port))
                .unwrap_or(("34020000000000000000", "0.0.0.0", 5060));
            
            let sip_msg = format!(
                "BYE sip:{}@{}:{} SIP/2.0\r\n\
                 Via: SIP/2.0/UDP {}:{};branch=z9hG4bK{};rport\r\n\
                 From: <sip:{}@{}:{}>;tag={}\r\n\
                 To: <sip:{}@{}:{}>;tag={}\r\n\
                 Call-ID: {}\r\n\
                 CSeq: 2 BYE\r\n\
                 User-Agent: RustCam-Media/2.0\r\n\
                 Content-Length: 0\r\n\r\n",
                device_tag, session.device_ip, session.device_audio_port,
                platform_ip, platform_port,
                chrono::Utc::now().timestamp_millis(),
                platform_id, platform_ip, platform_port, &session.from_tag,
                device_tag, session.device_ip, session.device_audio_port,
                session.to_tag.as_deref().unwrap_or(""),
                &session.call_id
            );
            
            if let Some(sender) = crate::protocol::gb28181::get_udp_sender() {
                let addr = format!("{}:{}", session.device_ip, session.device_audio_port);
                let _ = sender.send_to(sip_msg.as_bytes(), &addr).await;
            }
        }
        Ok(())
    }

    pub async fn send_audio_to_device(&self, device_tag: &str, pcm_data: &[i16]) -> Result<()> {
        let session_opt = {
            let mut sessions = self.audio_talk_sessions.write().await;
            if let Some(session) = sessions.get_mut(device_tag) {
                let encoded = crate::protocol::gb28181::audio::linear_to_alaw(pcm_data);
                let timestamp = session.rtp_timestamp;
                let ssrc = session.rtp_ssrc;
                let sequence = session.rtp_sequence;
                
                session.rtp_timestamp = session.rtp_timestamp.wrapping_add(160);
                session.rtp_sequence = session.rtp_sequence.wrapping_add(1);
                
                Some((encoded, timestamp, ssrc, sequence, session.device_ip.clone(), session.device_audio_port))
            } else {
                None
            }
        };
        
        if let Some((encoded, timestamp, ssrc, sequence, device_ip, device_port)) = session_opt {
            let rtp_packet = crate::protocol::gb28181::rtp::build_audio_rtp_packet_with_seq(
                &encoded,
                8,
                timestamp,
                ssrc,
                sequence,
            );
            
            let addr = format!("{}:{}", device_ip, device_port);
            if let Some(sender) = crate::protocol::gb28181::get_udp_sender() {
                sender.send_to(&rtp_packet, &addr).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            }
        }
        
        Ok(())
    }
    
    pub async fn send_audio_to_device_tcp(&self, device_tag: &str, pcm_data: &[i16]) -> Result<()> {
        let rtp_packet_with_len = {
            let mut sessions = self.audio_talk_sessions.write().await;
            if let Some(session) = sessions.get_mut(device_tag) {
                if session.tcp_stream.is_none() {
                    tracing::warn!("[GB28181] No TCP stream for device {}, attempting reconnect", device_tag);
                    let addr = format!("{}:{}", session.device_ip, session.device_audio_port);
                    if let Ok(stream) = tokio::net::TcpStream::connect(&addr).await {
                        session.tcp_stream = Some(Arc::new(tokio::sync::RwLock::new(stream)));
                        tracing::info!("[GB28181] TCP audio reconnected to {}", addr);
                    } else {
                        return Ok(());
                    }
                }
                
                let encoded = crate::protocol::gb28181::audio::linear_to_alaw(pcm_data);
                let timestamp = session.rtp_timestamp;
                let ssrc = session.rtp_ssrc;
                let sequence = session.rtp_sequence;
                
                session.rtp_timestamp = session.rtp_timestamp.wrapping_add(160);
                session.rtp_sequence = session.rtp_sequence.wrapping_add(1);
                
                let rtp_packet = crate::protocol::gb28181::rtp::build_audio_rtp_packet_with_seq(
                    &encoded, 8, timestamp, ssrc, sequence
                );
                
                let len = rtp_packet.len() as u16;
                let mut frame = Vec::with_capacity(2 + rtp_packet.len());
                frame.extend_from_slice(&len.to_be_bytes());
                frame.extend_from_slice(&rtp_packet);
                frame
            } else {
                return Ok(());
            }
        };
        
        if let Some(ref stream_arc) = self.audio_talk_sessions.read().await.get(device_tag).and_then(|s| s.tcp_stream.clone()) {
            let mut stream = stream_arc.write().await;
            if let Err(e) = stream.write_all(&rtp_packet_with_len).await {
                tracing::error!("[GB28181] TCP write failed: {}", e);
                if let Some(session) = self.audio_talk_sessions.write().await.get_mut(device_tag) {
                    session.tcp_stream = None;
                }
                return Err(anyhow::anyhow!("TCP write failed: {}", e).into());
            }
        }
        
        Ok(())
    }
    
    async fn send_bye_to_session(&self, session: &SubSession) -> Result<()> {
        let (platform_id, platform_ip, platform_port) = crate::protocol::gb28181::get_gb28181_platform_config()
            .get()
            .map(|(id, _, ip, port)| (id.as_str(), ip.as_str(), *port))
            .unwrap_or(("34020000000000000000", "0.0.0.0", 5060));

        let device_tag = self.device_tag_str();
        let via_transport = match session.transport {
            TransportProtocol::Tcp => "TCP",
            TransportProtocol::Udp => "UDP",
        };
        let sip_msg = format!(
            "BYE sip:{}@{}:{} SIP/2.0\r\n\
             Via: SIP/2.0/{} {}:{};branch=z9hG4bK{};rport\r\n\
             From: <sip:{}@{}:{}>;tag={}\r\n\
             To: <sip:{}@{}:{}>;tag={}\r\n\
             Call-ID: {}\r\n\
             CSeq: 2 BYE\r\n\
             User-Agent: RustCam-Media/2.0\r\n\
             Content-Length: 0\r\n\r\n",
            device_tag, session.device_host, session.device_port,
            via_transport, platform_ip, platform_port,
            chrono::Utc::now().timestamp_millis(),
            platform_id, platform_ip, platform_port, &session.from_tag,
            device_tag, session.device_host, session.device_port,
            session.to_tag.as_deref().unwrap_or(""),
            &session.call_id
        );

        match session.transport {
            TransportProtocol::Tcp => {
                if let Some(ref write_arc) = self.write {
                    let mut write = write_arc.write().await;
                    write.write_all(sip_msg.as_bytes()).await.map_err(|e| AppError::Internal(e.to_string()))?;
                    let _ = write.flush().await;
                }
            }
            TransportProtocol::Udp => {
                if let Some(sender) = crate::protocol::gb28181::get_udp_sender() {
                    let addr = format!("{}:{}", session.device_host, session.device_port);
                    sender.send_to(sip_msg.as_bytes(), &addr).await.map_err(|e| AppError::Internal(e.to_string()))?;
                }
            }
        }
        Ok(())
    }
}

fn rand_ssrc() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    (now.as_nanos() as u32).wrapping_add(12345)
}

fn rand_u16() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    (now.as_nanos() as u16).wrapping_add(1)
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    (now.as_nanos() as u32).wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ptz_cmd_compound_up_right() {
        let result = Gb28181Adapter::encode_ptz_cmd_compound(0.5, 0.5, 0.0, 50);
        assert!(result.contains("28"), "Up(0x08)+Right(0x20)=0x28, got: {}", result);
        assert!(result.contains("00 7F"), "No zoom (0), got: {}", result);
    }

    #[test]
    fn test_encode_ptz_cmd_compound_down_left() {
        let result = Gb28181Adapter::encode_ptz_cmd_compound(-0.5, -0.5, 0.0, 50);
        assert!(result.contains("12"), "Down(0x10)+Left(0x02)=0x12, got: {}", result);
    }

    #[test]
    fn test_encode_ptz_cmd_compound_with_zoom() {
        let result = Gb28181Adapter::encode_ptz_cmd_compound(0.5, 0.0, 0.3, 50);
        assert!(result.contains("20"), "Right(0x20), got: {}", result);
        assert!(result.contains("80"), "ZoomIn(0x80) in cmd2, got: {}", result);
    }

    #[test]
    fn test_encode_ptz_cmd_compound_all_stop() {
        let result = Gb28181Adapter::encode_ptz_cmd_compound(0.0, 0.0, 0.0, 50);
        assert!(result.contains("00 00 00 00 00"), "All stop should be 00s, got: {}", result);
    }

    #[test]
    fn test_encode_ptz_cmd_single_up() {
        let result = Gb28181Adapter::encode_ptz_cmd("up", 50);
        assert!(result.contains("08"), "Up should set cmd1=0x08, got: {}", result);
    }

    #[test]
    fn test_encode_ptz_cmd_single_right() {
        let result = Gb28181Adapter::encode_ptz_cmd("right", 50);
        assert!(result.contains("40"), "Right should set cmd1=0x40, got: {}", result);
    }

    #[test]
    fn test_encode_ptz_cmd_zoom_in() {
        let result = Gb28181Adapter::encode_ptz_cmd("zoom_in", 50);
        assert!(result.contains("80"), "ZoomIn should set cmd2=0x80, got: {}", result);
    }

    #[test]
    fn test_datetime_to_ntp_string() {
        use chrono::{TimeZone, Utc};
        let dt = Utc.with_ymd_and_hms(2024, 8, 7, 14, 30, 0).unwrap();
        let result = Gb28181Adapter::datetime_to_ntp_string(dt);
        assert_eq!(result, "20240807143000");
    }

    #[test]
    fn test_build_ptz_xml_compound() {
        let result = Gb28181Adapter::build_ptz_xml_compound("34020000001110000001", 0.5, 0.5, 0.0, 50);
        assert!(result.contains("34020000001110000001"));
        assert!(result.contains("PTZCmd"));
    }

    #[tokio::test]
    async fn test_build_catalog_query() {
        use std::sync::Once;
        static SETUP: Once = Once::new();
        SETUP.call_once(|| {
            crate::protocol::gb28181::set_gb28181_platform_config(
                "31011500001000000001".to_string(),
                "3101150000".to_string(),
                "192.168.1.100".to_string(),
                5060,
            );
        });

        let deps = ProtocolDeps::for_test();
        
        let mut adapter = Gb28181Adapter::new(deps, Arc::new(|_, _| {}), Arc::new(|_| {}));
        adapter.device_tag = Some("12010100001320000011".to_string());
        adapter.udp_peer = Some("192.168.1.211:5060".parse().unwrap());

        let query = adapter.build_catalog_query();
        let query_str = String::from_utf8_lossy(&query);
        
        assert!(query_str.contains("MESSAGE sip:"), "Should contain MESSAGE sip:");
        assert!(query_str.contains("12010100001320000011"), "Should contain device ID");
        assert!(query_str.contains("CmdType>Catalog"), "Should contain Catalog command");
        assert!(query_str.contains("Via:"), "Should contain Via header");
        assert!(query_str.contains("CSeq:"), "Should contain CSeq header");
        assert!(query_str.contains("From:"), "Should contain From header");
        assert!(query_str.contains("To:"), "Should contain To header");
        assert!(query_str.contains("Call-ID:"), "Should contain Call-ID header");
    }
}
