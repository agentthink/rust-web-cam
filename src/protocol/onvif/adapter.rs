use std::net::SocketAddr;
use async_trait::async_trait;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::AsyncWriteExt;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::protocol::adapter::SignalAdapter;
use crate::protocol::event::{SignalEvent, ProtocolType, TransportType, PtzCommand};
use crate::protocol::traits::ProtocolDeps;
use crate::protocol::onvif::auth::UsernameToken;
use crate::protocol::onvif::events::PullPointServer;
use crate::error::{Result, AppError};

// ═══════════════════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════════════════

/// 从字节中提取本地名称（去除命名空间前缀）
fn local_name(name_bytes: &[u8]) -> &str {
    let name_str = std::str::from_utf8(name_bytes).unwrap_or("");
    if let Some(idx) = name_str.find(':') {
        &name_str[idx + 1..]
    } else {
        name_str
    }
}

// ═══════════════════════════════════════════════════════════════
// ONVIF 设备信息
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Default)]
pub struct OnvifDeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
    pub password: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
// ONVIF 适配器
// ═══════════════════════════════════════════════════════════════

pub struct OnvifAdapter {
    device_id: Option<String>,
    remote_addr: Option<SocketAddr>,
    recv_buffer: Vec<u8>,
    device_info: OnvifDeviceInfo,
    pull_point_server: PullPointServer,
    rtsp_host: Option<String>,
    rtsp_port: Option<u16>,
    write: Option<std::sync::Arc<tokio::sync::RwLock<OwnedWriteHalf>>>,
    deps: ProtocolDeps,
}

impl OnvifAdapter {
    pub fn new(deps: ProtocolDeps) -> Self {
        // ONVIF 模拟设备默认使用 RTSP 端口 8554
        let rtsp_port = deps
            .config
            .media_servers
            .servers
            .first()
            .and_then(|s| s.protocol_ports.rtsp)
            .unwrap_or(8554);

        Self {
            device_id: None,
            remote_addr: None,
            recv_buffer: Vec::new(),
            device_info: OnvifDeviceInfo {
                manufacturer: "RustCam".to_string(),
                model: "ONVIF-NVT".to_string(),
                firmware_version: "2.0.0".to_string(),
                serial_number: "00000001".to_string(),
                hardware_id: "rustcam-onvif".to_string(),
                password: Some("admin".to_string()),
            },
            pull_point_server: PullPointServer::new(),
            rtsp_host: Some(deps.config.server.host.clone()),
            rtsp_port: Some(rtsp_port),
            write: None,
            deps,
        }
    }

    pub fn with_rtsp_server(mut self, host: String, port: u16) -> Self {
        self.rtsp_host = Some(host);
        self.rtsp_port = Some(port);
        self
    }

    pub fn with_device_info(mut self, info: OnvifDeviceInfo) -> Self {
        self.device_id = Some(info.serial_number.clone());
        self.device_info = info;
        self
    }

    // ═══════════════════════════════════════════════════════
    // 消息解析
    // ═══════════════════════════════════════════════════════

    fn extract_message(buffer: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        let header_end = memchr::memmem::find(buffer, b"\r\n\r\n")?;
        let header_str = String::from_utf8_lossy(&buffer[..header_end]);

        let content_length = header_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let body_start = header_end + 4;
        let total_len = body_start + content_length;
        if buffer.len() < total_len {
            return None;
        }

        Some((buffer[..total_len].to_vec(), buffer[total_len..].to_vec()))
    }

    fn get_soap_action(headers: &str) -> Option<String> {
        headers
            .lines()
            .find(|l| l.to_lowercase().starts_with("soapaction:"))
            .and_then(|l| l.split(':').nth(1))
            .map(|v| v.trim().trim_matches('"').to_string())
    }

    // ═══════════════════════════════════════════════════════
    // SOAP 响应构建
    // ═══════════════════════════════════════════════════════

    fn build_soap_response(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body>{}</s:Body>
</s:Envelope>"#,
            body
        )
    }

    fn build_soap_fault(fault_code: &str, fault_subcode: &str, fault_string: &str) -> String {
        Self::build_soap_response(&format!(
            r#"<s:Fault>
  <s:Code><s:Value>{}</s:Value><s:Subcode><s:Value>{}</s:Value></s:Subcode></s:Code>
  <s:Reason><s:Text>{}</s:Text></s:Reason>
</s:Fault>"#,
            fault_code, fault_subcode, fault_string
        ))
    }

    fn build_get_device_info_response(&self) -> String {
        let body = format!(
            r#"<GetDeviceInformationResponse xmlns="http://www.onvif.org/ver10/device/wsdl">
<Manufacturer>{}</Manufacturer>
<Model>{}</Model>
<FirmwareVersion>{}</FirmwareVersion>
<SerialNumber>{}</SerialNumber>
<HardwareId>{}</HardwareId>
</GetDeviceInformationResponse>"#,
            self.device_info.manufacturer,
            self.device_info.model,
            self.device_info.firmware_version,
            self.device_info.serial_number,
            self.device_info.hardware_id,
        );
        Self::build_soap_response(&body)
    }

    fn build_get_capabilities_response(&self) -> String {
        let host = self.rtsp_host.as_deref().unwrap_or("127.0.0.1");
        let port = self.rtsp_port.unwrap_or(8554);
        let body = format!(
            r#"<GetCapabilitiesResponse xmlns="http://www.onvif.org/ver10/device/wsdl">
<Capabilities>
  <Device><XAddr>http://{}:{}/onvif/device_service</XAddr></Device>
  <Media><XAddr>http://{}:{}/onvif/media_service</XAddr>
    <StreamingCapabilities>
      <RTPMulticast>true</RTPMulticast>
      <RTP_TCP>true</RTP_TCP>
      <RTP_RTSP_TCP>true</RTP_RTSP_TCP>
    </StreamingCapabilities>
  </Media>
  <PTZ><XAddr>http://{}:{}/onvif/ptz_service</XAddr></PTZ>
  <Events><XAddr>http://{}:{}/onvif/event_service</XAddr>
    <WSSubscriptionPolicySupport>true</WSSubscriptionPolicySupport>
    <WSPullPointSupport>true</WSPullPointSupport>
    <WSPausableSubscriptionManagerInterfaceSupport>true</WSPausableSubscriptionManagerInterfaceSupport>
  </Events>
</Capabilities>
</GetCapabilitiesResponse>"#,
            host, port, host, port, host, port, host, port,
        );
        Self::build_soap_response(&body)
    }

    fn build_get_profiles_response(&self) -> String {
        let body = r#"<GetProfilesResponse xmlns="http://www.onvif.org/ver10/media/wsdl">
<Profiles token="Profile_1" fixed="true">
  <Name>MainStream</Name>
  <VideoSourceConfiguration token="VideoSource_1">
    <Name>VideoSource</Name><UseCount>1</UseCount>
    <SourceToken>VideoSource_1</SourceToken>
    <Bounds x="0" y="0" width="1920" height="1080"/>
  </VideoSourceConfiguration>
  <VideoEncoderConfiguration token="VideoEncoder_1">
    <Name>H264_1080P</Name><UseCount>1</UseCount>
    <Encoding>H264</Encoding>
    <Resolution><Width>1920</Width><Height>1080</Height></Resolution>
    <RateControl><FrameRateLimit>30</FrameRateLimit><BitrateLimit>4096</BitrateLimit></RateControl>
  </VideoEncoderConfiguration>
  <PTZConfiguration token="PTZ_1">
    <Name>PTZ</Name><UseCount>1</UseCount><NodeToken>PTZNode_1</NodeToken>
  </PTZConfiguration>
</Profiles>
<Profiles token="Profile_2" fixed="false">
  <Name>SubStream</Name>
  <VideoSourceConfiguration token="VideoSource_2">
    <Name>VideoSource</Name><UseCount>1</UseCount>
    <SourceToken>VideoSource_1</SourceToken>
    <Bounds x="0" y="0" width="1280" height="720"/>
  </VideoSourceConfiguration>
  <VideoEncoderConfiguration token="VideoEncoder_2">
    <Name>H264_720P</Name><UseCount>1</UseCount>
    <Encoding>H264</Encoding>
    <Resolution><Width>1280</Width><Height>720</Height></Resolution>
    <RateControl><FrameRateLimit>25</FrameRateLimit><BitrateLimit>2048</BitrateLimit></RateControl>
  </VideoEncoderConfiguration>
</Profiles>
</GetProfilesResponse>"#;
        Self::build_soap_response(body)
    }

    fn build_get_stream_uri_response(&self, profile_token: &str) -> String {
        let host = self.rtsp_host.as_deref().unwrap_or("127.0.0.1");
        let port = self.rtsp_port.unwrap_or(8554);
        let stream_key = format!("live/{}", profile_token);
        let rtsp_uri = format!("rtsp://{}:{}/{}", host, port, stream_key);
        let body = format!(
            r#"<GetStreamUriResponse xmlns="http://www.onvif.org/ver10/media/wsdl">
<MediaUri>
  <Uri>{}</Uri>
  <InvalidAfterConnect>false</InvalidAfterConnect>
  <InvalidAfterReboot>false</InvalidAfterReboot>
  <Timeout>PT0S</Timeout>
</MediaUri>
</GetStreamUriResponse>"#,
            rtsp_uri
        );
        Self::build_soap_response(&body)
    }

    fn build_get_snapshot_uri_response(&self) -> String {
        let device_id = self.device_id.as_deref().unwrap_or("onvif");
        let host = self.rtsp_host.as_deref().unwrap_or("127.0.0.1");
        let port = self.rtsp_port.unwrap_or(8080);
        let body = format!(
            r#"<GetSnapshotUriResponse xmlns="http://www.onvif.org/ver10/media/wsdl">
<MediaUri><Uri>http://{}:{}/snapshot/{}.jpg</Uri></MediaUri>
</GetSnapshotUriResponse>"#,
            host, port, device_id,
        );
        Self::build_soap_response(&body)
    }

    fn build_get_video_sources_response(&self) -> String {
        let body = r#"<GetVideoSourcesResponse xmlns="http://www.onvif.org/ver10/media/wsdl">
<VideoSources token="VideoSource_1">
  <Framerate>30</Framerate>
  <Resolution><Width>1920</Width><Height>1080</Height></Resolution>
</VideoSources>
</GetVideoSourcesResponse>"#;
        Self::build_soap_response(body)
    }

    fn build_ptz_response() -> String {
        Self::build_soap_response(
            r#"<ContinuousMoveResponse xmlns="http://www.onvif.org/ver20/ptz/wsdl"/>"#,
        )
    }

    fn build_get_system_date_time_response(&self) -> String {
        use chrono::Local;
        let now = Local::now();
        let body = format!(
            r#"<GetSystemDateAndTimeResponse xmlns="http://www.onvif.org/ver10/device/wsdl">
<SystemDateAndTime>
  <DateTimeType>Manual</DateTimeType>
  <DaylightSavings>false</DaylightSavings>
  <TimeZone><TZ>UTC+8</TZ></TimeZone>
  <UTCDateTime>
    <Time><Hour>{}</Hour><Minute>{}</Minute><Second>{}</Second></Time>
    <Date><Year>{}</Year><Month>{}</Month><Day>{}</Day></Date>
  </UTCDateTime>
</SystemDateAndTime>
</GetSystemDateAndTimeResponse>"#,
            now.format("%H"),
            now.format("%M"),
            now.format("%S"),
            now.format("%Y"),
            now.format("%m"),
            now.format("%d"),
        );
        Self::build_soap_response(&body)
    }

    fn build_get_network_interfaces_response(&self) -> String {
        let host = self.rtsp_host.as_deref().unwrap_or("192.168.1.100");
        let body = format!(
            r#"<GetNetworkInterfacesResponse xmlns="http://www.onvif.org/ver10/device/wsdl">
<NetworkInterfaces token="eth0">
  <Enabled>true</Enabled>
  <Info><Name>eth0</Name><HwAddress>00:00:00:00:00:00</HwAddress><MTU>1500</MTU></Info>
  <IPv4><Enabled>true</Enabled>
    <Config><Manual><Address>{}</Address><PrefixLength>24</PrefixLength></Manual><DHCP>false</DHCP></Config>
  </IPv4>
</NetworkInterfaces>
</GetNetworkInterfacesResponse>"#,
            host
        );
        Self::build_soap_response(&body)
    }

    fn build_get_users_response(&self) -> String {
        let body = r#"<GetUsersResponse xmlns="http://www.onvif.org/ver10/device/wsdl">
<User><Username>admin</Username><UserLevel>Administrator</UserLevel></User>
<User><Username>operator</Username><UserLevel>Operator</UserLevel></User>
</GetUsersResponse>"#;
        Self::build_soap_response(body)
    }

    // ═══════════════════════════════════════════════════════
    // XML 解析辅助方法
    // ═══════════════════════════════════════════════════════

    fn extract_element_text(data: &[u8], element: &str) -> Option<String> {
        let mut reader = Reader::from_reader(data);
        reader.config_mut().trim_text(true);
        let mut capture = false;
        let mut result = String::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().local_name();
                    if name.as_ref() == element {
                        capture = true;
                    }
                }
                Ok(Event::Text(ref e)) if capture => {
                    result.push_str(&*e);
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().local_name();
                    if name.as_ref() == element {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result.trim().to_string())
        }
    }

    fn extract_profile_token(data: &[u8]) -> String {
        let mut reader = Reader::from_reader(data);
        reader.config_mut().trim_text(true);
        let mut text_content = String::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().local_name();
                    if name.as_ref() == "ProfileToken" || name.as_ref() == "Profile" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == "token" {
                                return attr.value.to_string();
                            }
                        }
                    }
                }
                Ok(Event::Text(ref e)) => {
                    text_content = (&*e).to_string();
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().local_name();
                    if name.as_ref() == "ProfileToken" && !text_content.is_empty() {
                        return text_content.clone();
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        "Profile_1".to_string()
    }

    fn parse_ptz_velocity(data: &[u8]) -> (f64, f64, f64) {
        let mut reader = Reader::from_reader(data);
        reader.config_mut().trim_text(true);
        let mut pan: f64 = 0.0;
        let mut tilt: f64 = 0.0;
        let mut zoom: f64 = 0.0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().local_name();
                    if name.as_ref() == "PanTilt" || name.as_ref() == "Zoom" {
                        for attr in e.attributes().flatten() {
                            let key = attr.key.as_ref();
                            if let Ok(val) = attr.value.parse::<f64>() {
                                match key {
                                    "x" => {
                                        if name.as_ref() == "Zoom" {
                                            zoom = val;
                                        } else {
                                            pan = val;
                                        }
                                    }
                                    "y" => tilt = val,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        (pan, tilt, zoom)
    }

    fn parse_duration_hours(termination: &str) -> i64 {
        if termination.starts_with("PT") {
            let rest = &termination[2..];
            if let Some(h) = rest.strip_suffix('H') {
                return h.parse().unwrap_or(24);
            }
            if let Some(m) = rest.strip_suffix('M') {
                let mins: i64 = m.parse().unwrap_or(1440);
                return mins / 60 + 1;
            }
            if let Some(s) = rest.strip_suffix('S') {
                let secs: i64 = s.parse().unwrap_or(86400);
                return secs / 3600 + 1;
            }
        }
        24
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
    // SOAP 请求处理
    // ═══════════════════════════════════════════════════════

    fn handle_soap_action(&self, action: &str, data: &[u8]) -> Option<String> {
        if action.contains("GetDeviceInformation") {
            Some(self.build_get_device_info_response())
        } else if action.contains("GetCapabilities") {
            Some(self.build_get_capabilities_response())
        } else if action.contains("GetProfiles") {
            Some(self.build_get_profiles_response())
        } else if action.contains("GetStreamUri") {
            let profile = Self::extract_profile_token(data);
            Some(self.build_get_stream_uri_response(&profile))
        } else if action.contains("GetSnapshotUri") {
            Some(self.build_get_snapshot_uri_response())
        } else if action.contains("GetVideoSources") {
            Some(self.build_get_video_sources_response())
        } else if action.contains("ContinuousMove")
            || action.contains("AbsoluteMove")
            || action.contains("RelativeMove")
            || action.contains("Stop")
        {
            Some(Self::build_ptz_response())
        } else if action.contains("GetPresets") {
            let presets = self.pull_point_server.get_presets();
            Some(PullPointServer::build_get_presets_response(&presets))
        } else if action.contains("SetPreset") {
            let name = Self::extract_element_text(data, "PresetName")
                .unwrap_or_else(|| "Preset".to_string());
            let token = self.pull_point_server.save_preset(name);
            let body = format!(
                r#"<SetPresetResponse xmlns="http://www.onvif.org/ver20/ptz/wsdl"><PresetToken>{}</PresetToken></SetPresetResponse>"#,
                token
            );
            Some(Self::build_soap_response(&body))
        } else if action.contains("RemovePreset") {
            let token =
                Self::extract_element_text(data, "PresetToken").unwrap_or_default();
            self.pull_point_server.remove_preset(&token);
            Some(Self::build_soap_response(
                r#"<RemovePresetResponse xmlns="http://www.onvif.org/ver20/ptz/wsdl"/>"#,
            ))
        } else if action.contains("GotoPreset") {
            Some(Self::build_soap_response(
                r#"<GotoPresetResponse xmlns="http://www.onvif.org/ver20/ptz/wsdl"/>"#,
            ))
        } else if action.contains("GetStatus") {
            let status = r#"<GetStatusResponse xmlns="http://www.onvif.org/ver20/ptz/wsdl">
  <PTZStatus>
    <Position><PanTilt x="0.0" y="0.0"/><Zoom x="1.0"/></Position>
    <MoveStatus><PanTilt>IDLE</PanTilt><Zoom>IDLE</Zoom></MoveStatus>
  </PTZStatus>
</GetStatusResponse>"#;
            Some(Self::build_soap_response(status))
        } else if action.contains("CreatePullPointSubscription") {
            let timeout = Self::extract_element_text(data, "TerminationTime")
                .as_deref()
                .map(Self::parse_duration_hours)
                .unwrap_or(24);
            let sub_ref = self
                .pull_point_server
                .create_subscription_with_timeout(timeout);
            Some(PullPointServer::build_create_subscription_response(
                &sub_ref,
            ))
        } else if action.contains("PullMessages") {
            let sub_ref = Self::extract_element_text(data, "SubscriptionReference")
                .unwrap_or_else(|| "urn:uuid:unknown".to_string());
            let messages = self.pull_point_server.pull_messages(&sub_ref, 100, 5);
            Some(PullPointServer::build_pull_messages_response(
                &messages,
                &sub_ref,
            ))
        } else if action.contains("GetEventProperties") {
            Some(PullPointServer::build_get_event_properties_response())
        } else if action.contains("Renew") {
            let sub_ref = Self::extract_element_text(data, "SubscriptionReference")
                .unwrap_or_default();
            if let Some(termination) = self.pull_point_server.renew_subscription(&sub_ref, 24) {
                Some(PullPointServer::build_renew_response(&termination))
            } else {
                Some(Self::build_soap_fault(
                    "s:Receiver",
                    "wsa:InvalidMessageReference",
                    "Subscription not found",
                ))
            }
        } else if action.contains("Unsubscribe") {
            let sub_ref = Self::extract_element_text(data, "SubscriptionReference")
                .unwrap_or_default();
            self.pull_point_server.unsubscribe(&sub_ref);
            Some(PullPointServer::build_unsubscribe_response())
        } else if action.contains("GetSystemDateAndTime") {
            Some(self.build_get_system_date_time_response())
        } else if action.contains("GetNetworkInterfaces") {
            Some(self.build_get_network_interfaces_response())
        } else if action.contains("GetUsers") {
            Some(self.build_get_users_response())
        } else if action.contains("CreateUsers") || action.contains("DeleteUsers") {
            Some(Self::build_soap_response(
                r#"<CreateUsersResponse xmlns="http://www.onvif.org/ver10/device/wsdl"/>"#,
            ))
        } else if action.contains("GetNetworkDefaultSettings") {
            Some(Self::build_soap_response(
                r#"<GetNetworkDefaultSettingsResponse xmlns="http://www.onvif.org/ver10/device/wsdl">
  <IPv4><Enabled>true</Enabled><Config><DHCP>true</DHCP></Config></IPv4>
</GetNetworkDefaultSettingsResponse>"#,
            ))
        } else {
            tracing::debug!("[ONVIF] Unhandled action: {}", action);
            Some(Self::build_soap_fault(
                "s:Client",
                "ter:ActionNotSupported",
                &format!("Unsupported action: {}", action),
            ))
        }
    }

    fn parse_message(
        &mut self,
        data: &[u8],
    ) -> Result<(Vec<SignalEvent>, Option<Vec<u8>>)> {
        let header_end = memchr::memmem::find(data, b"\r\n\r\n")
            .ok_or_else(|| AppError::Internal("ONVIF: no header end".to_string()))?;
        let header_str = String::from_utf8_lossy(&data[..header_end]);

        let soap_action = Self::get_soap_action(&header_str);
        let device_id = Self::extract_element_text(data, "SerialNumber")
            .or_else(|| Self::extract_element_text(data, "Address"))
            .or_else(|| self.device_id.clone())
            .unwrap_or_else(|| "onvif_device".to_string());

        if self.device_id.is_none() {
            self.device_id = Some(device_id.clone());
        }

        let mut events = Vec::new();

        // 认证检查
        if let Some(ref action) = soap_action {
            if let Some(token) = UsernameToken::from_xml(data) {
                let password = self.device_info.password.as_deref().unwrap_or("admin");
                if !token.verify(password) {
                    tracing::warn!("[ONVIF] Auth failed for user: {}", token.username);
                    let fault = Self::build_soap_fault(
                        "env:Sender",
                        "ter:NotAuthenticated",
                        "Authentication failed",
                    );
                    return Ok((events, Some(fault.into_bytes())));
                }
            }

            if let Some(response) = self.handle_soap_action(action, data) {
                return Ok((events, Some(response.into_bytes())));
            }
        }

        // 生成事件
        if let Some(action) = soap_action.as_deref() {
            if action.contains("GetDeviceInformation") || action.contains("GetCapabilities") {
                events.push(SignalEvent::QueryDeviceInfo {
                    device_id: 0,
                    device_tag: None,
                });
            }
            if action.contains("GetStreamUri") || action.contains("GetSnapshotUri") {
                events.push(SignalEvent::StartPlay {
                    device_id: 0,
                    device_tag: None,
                    session_id: format!("onvif_{}", chrono::Utc::now().timestamp_millis()),
                    channel_id: Some(Self::extract_profile_token(data)),
                    transport: TransportType::TCP,
                    media_server_name: None,
                });
            }
            if action.contains("ContinuousMove") {
                let (pan, tilt, zoom) = Self::parse_ptz_velocity(data);
                let speed =
                    ((pan.abs().max(tilt.abs()).max(zoom.abs()) * 100.0) as u8).clamp(1, 100);
                events.push(SignalEvent::PtzControl {
                    device_id: device_id.clone(),
                    command: PtzCommand::ContinuousMove { pan, tilt, zoom },
                    speed: Some(speed),
                });
            }
            if action.contains("AbsoluteMove") {
                let (pan, tilt, zoom) = Self::parse_ptz_velocity(data);
                events.push(SignalEvent::PtzControl {
                    device_id: device_id.clone(),
                    command: PtzCommand::AbsoluteMove { pan, tilt, zoom },
                    speed: Some(50),
                });
            }
            if action.contains("RelativeMove") {
                let (pan, tilt, zoom) = Self::parse_ptz_velocity(data);
                events.push(SignalEvent::PtzControl {
                    device_id: device_id.clone(),
                    command: PtzCommand::RelativeMove { pan, tilt, zoom },
                    speed: Some(50),
                });
            }
            if action.contains("Stop") {
                events.push(SignalEvent::PtzControl {
                    device_id: device_id.clone(),
                    command: PtzCommand::Stop,
                    speed: Some(0),
                });
            }
        }

        Ok((events, None))
    }
}

// ═══════════════════════════════════════════════════════════════
// SignalAdapter Trait Implementation
// ═══════════════════════════════════════════════════════════════

#[async_trait]
impl SignalAdapter for OnvifAdapter {
    async fn parse(&mut self, data: &[u8]) -> Result<Vec<SignalEvent>> {
        self.recv_buffer.extend_from_slice(data);
        let mut events = Vec::new();

        loop {
            let buffer = self.recv_buffer.clone();
            let (msg, remainder) = match Self::extract_message(&buffer) {
                Some((msg, rem)) if !msg.is_empty() => (msg, rem),
                _ => break,
            };
            self.recv_buffer = remainder;

            let (msg_events, response) = self.parse_message(&msg)?;
            events.extend(msg_events);

            if let Some(resp) = response {
                self.send_raw(&resp).await?;
            }
        }

        Ok(events)
    }

    async fn on_connected(&mut self, addr: SocketAddr) -> Result<()> {
        self.remote_addr = Some(addr);
        tracing::info!("[ONVIF] Connection from {}", addr);
        Ok(())
    }

    async fn on_disconnected(&mut self) -> Result<()> {
        let device_id = self.device_id.as_deref().unwrap_or("unknown");
        tracing::info!("[ONVIF] Device {} disconnected", device_id);
        let _ = self
            .deps
            .device_lookup
            .set_offline(device_id, Some("Connection closed"))
            .await;
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.send_raw(data).await
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Onvif
    }

    fn name(&self) -> &'static str {
        "ONVIF"
    }

    fn keepalive(&self) -> bool {
        true
    }

    fn idle_timeout(&self) -> Option<u64> {
        Some(120)
    }

    fn set_tcp_write(&mut self, write: OwnedWriteHalf) {
        self.write = Some(std::sync::Arc::new(tokio::sync::RwLock::new(write)));
    }

    async fn start(&mut self, _device_tag: &str) -> Result<()> { Ok(()) }
    async fn ptz_control(&mut self, _channel_id: &str, _command: &crate::protocol::event::PtzCommand, _speed: Option<u8>) -> Result<()> { Ok(()) }
}