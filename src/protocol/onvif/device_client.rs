use crate::protocol::onvif::auth::UsernameToken;
use crate::protocol::onvif::soap::{extract_element_text, extract_attribute};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct OnvifDeviceInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub serial_number: Option<String>,
    pub hardware_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OnvifProfile {
    pub token: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct OnvifStreamUri {
    pub uri: String,
    pub invalid_after_connect: bool,
    pub invalid_after_reboot: bool,
    pub timeout: String,
}

#[derive(Debug, Clone, Default)]
pub struct OnvifCapabilities {
    pub media: Option<String>,
    pub ptz: Option<String>,
    pub events: Option<String>,
    pub imaging: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OnvifServiceEndpoint {
    pub namespace: String,
    pub xaddr: String,
}

pub struct OnvifDeviceClient {
    x_addr: String,
    username: Option<String>,
    password: Option<String>,
}

impl OnvifDeviceClient {
    pub fn new(x_addr: &str) -> Self {
        Self {
            x_addr: x_addr.to_string(),
            username: None,
            password: None,
        }
    }

    pub fn with_credentials(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    /// 构建带 WS-Security 认证的 SOAP 信封
    fn build_soap_envelope(&self, body: &str) -> String {
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            let (nonce, created, digest) = UsernameToken::build_digest(u, p);
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
<s:Header>
  <wsse:Security s:mustUnderstand="true">
    <wsse:UsernameToken>
      <wsse:Username>{}</wsse:Username>
      <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{}</wsse:Password>
      <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{}</wsse:Nonce>
      <wsu:Created>{}</wsu:Created>
    </wsse:UsernameToken>
  </wsse:Security>
</s:Header>
<s:Body>{}</s:Body>
</s:Envelope>"#,
                u, digest, nonce, created, body
            )
        } else {
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body>{}</s:Body>
</s:Envelope>"#,
                body
            )
        }
    }

    /// 发送 SOAP 请求
    async fn send_soap(&self, body: &str) -> anyhow::Result<String> {
        let envelope = self.build_soap_envelope(body);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let resp = client
            .post(&self.x_addr)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(envelope)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP {} from {}",
                resp.status(),
                self.x_addr
            ));
        }

        Ok(resp.text().await?)
    }

    // ═══════════════════════════════════════════════════════════
    // 设备信息
    // ═══════════════════════════════════════════════════════════

    pub async fn get_device_info(&self) -> anyhow::Result<OnvifDeviceInfo> {
        let body = r#"<GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>"#;
        let xml = self.send_soap(body).await?;

        Ok(OnvifDeviceInfo {
            manufacturer: extract_element_text(xml.as_bytes(), "Manufacturer"),
            model: extract_element_text(xml.as_bytes(), "Model"),
            firmware_version: extract_element_text(xml.as_bytes(), "FirmwareVersion"),
            serial_number: extract_element_text(xml.as_bytes(), "SerialNumber"),
            hardware_id: extract_element_text(xml.as_bytes(), "HardwareId"),
        })
    }

    // ═══════════════════════════════════════════════════════════
    // 获取 Profiles
    // ═══════════════════════════════════════════════════════════

    pub async fn get_profiles(&self) -> anyhow::Result<Vec<OnvifProfile>> {
        let body = r#"<GetProfiles xmlns="http://www.onvif.org/ver10/media/wsdl"/>"#;
        let xml = self.send_soap(body).await?;

        let mut profiles = Vec::new();
        let mut current_token = String::new();
        let mut current_name = String::new();
        let mut in_profile = false;
        let mut in_token = false;
        let mut in_name = false;

        for line in xml.lines() {
            let line = line.trim();

            if line.contains("<Profiles") && line.contains("token=") {
                in_profile = true;
                if let Some(start) = line.find("token=\"") {
                    let rest = &line[start + 7..];
                    if let Some(end) = rest.find('"') {
                        current_token = rest[..end].to_string();
                    }
                }
            }

            if in_profile && line.contains("<Name>") {
                in_name = true;
                if let Some(start) = line.find("<Name>") {
                    let rest = &line[start + 6..];
                    if let Some(end) = rest.find("</Name>") {
                        current_name = rest[..end].to_string();
                    }
                }
            }

            if line.contains("</Profiles>") && in_profile {
                if !current_token.is_empty() {
                    profiles.push(OnvifProfile {
                        token: current_token.clone(),
                        name: current_name.clone(),
                    });
                }
                current_token.clear();
                current_name.clear();
                in_profile = false;
            }
        }

        Ok(profiles)
    }

    // ═══════════════════════════════════════════════════════════
    // 获取流 URI（RTSP 地址）
    // ═══════════════════════════════════════════════════════════

    pub async fn get_stream_uri(&self, profile_token: &str) -> anyhow::Result<OnvifStreamUri> {
        let body = format!(
            r#"<GetStreamUri xmlns="http://www.onvif.org/ver10/media/wsdl">
  <StreamSetup>
    <Stream xmlns="http://www.onvif.org/ver10/schema">RTP-Unicast</Stream>
    <Transport xmlns="http://www.onvif.org/ver10/schema">
      <Protocol>RTSP</Protocol>
    </Transport>
  </StreamSetup>
  <ProfileToken>{}</ProfileToken>
</GetStreamUri>"#,
            profile_token
        );
        let xml = self.send_soap(&body).await?;

        let uri = extract_element_text(xml.as_bytes(), "Uri")
            .unwrap_or_default();

        Ok(OnvifStreamUri {
            uri,
            invalid_after_connect: false,
            invalid_after_reboot: false,
            timeout: String::new(),
        })
    }

    // ═══════════════════════════════════════════════════════════
    // 便捷方法：获取第一个 Profile 的 RTSP 地址
    // ═══════════════════════════════════════════════════════════

    pub async fn get_first_rtsp_url(&self) -> anyhow::Result<String> {
        let profiles = self.get_profiles().await?;
        if profiles.is_empty() {
            return Err(anyhow::anyhow!("No profiles found"));
        }

        let stream_uri = self.get_stream_uri(&profiles[0].token).await?;
        Ok(stream_uri.uri)
    }

    pub async fn get_capabilities(&self) -> anyhow::Result<OnvifCapabilities> {
        let body = r#"<GetCapabilities xmlns="http://www.onvif.org/ver10/device/wsdl"/>"#;
        let xml = self.send_soap(body).await?;

        let caps = OnvifCapabilities {
            media: Self::extract_capability_url(&xml, "Media", "XAddr"),
            ptz: Self::extract_capability_url(&xml, "PTZ", "XAddr"),
            events: Self::extract_capability_url(&xml, "Events", "XAddr"),
            imaging: Self::extract_capability_url(&xml, "Imaging", "XAddr"),
        };

        Ok(caps)
    }

    pub async fn get_services(&self) -> anyhow::Result<Vec<OnvifServiceEndpoint>> {
        let body = r#"<GetServices xmlns="http://www.onvif.org/ver10/device/wsdl"><IncludeCapability>false</IncludeCapability></GetServices>"#;
        let xml = self.send_soap(body).await?;
        Ok(Self::parse_service_endpoints(&xml))
    }

    pub async fn get_capabilities_with_fallback(&self) -> anyhow::Result<OnvifCapabilities> {
        if let Ok(services) = self.get_services().await {
            if !services.is_empty() {
                let mut caps = OnvifCapabilities::default();
                for svc in services {
                    let ns = svc.namespace.to_lowercase();
                    let url = svc.xaddr;
                    if ns.contains("media") && caps.media.is_none() {
                        caps.media = Some(url);
                    } else if ns.contains("ptz") && caps.ptz.is_none() {
                        caps.ptz = Some(url);
                    } else if ns.contains("events") && caps.events.is_none() {
                        caps.events = Some(url);
                    } else if ns.contains("imaging") && caps.imaging.is_none() {
                        caps.imaging = Some(url);
                    }
                }
                if caps.media.is_some() || caps.ptz.is_some() || caps.events.is_some() || caps.imaging.is_some() {
                    return Ok(caps);
                }
            }
        }
        self.get_capabilities().await
    }

    fn parse_service_endpoints(xml: &str) -> Vec<OnvifServiceEndpoint> {
        let mut endpoints = Vec::new();
        let bytes = xml.as_bytes();
        let mut reader = quick_xml::Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut current_ns = String::new();
        let mut current_xaddr = String::new();
        let mut in_service = false;

        loop {
            use quick_xml::events::Event;
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = e.name().local_name().into_inner();
                    if name == "Service" {
                        in_service = true;
                        current_ns.clear();
                        current_xaddr.clear();
                        for attr in e.attributes().flatten() {
                            let key = attr.key.into_inner();
                            if key == "Namespace" {
                                current_ns = attr.value.to_string();
                            }
                        }
                    }
                    if in_service && name == "XAddr" {
                        let mut text = String::new();
                        loop {
                            match reader.read_event() {
                                Ok(Event::Text(ref e)) => text.push_str(&*e),
                                Ok(Event::End(ref e)) => {
                                    if e.name().local_name().into_inner() == "XAddr" { break; }
                                }
                                Ok(Event::Eof) | Err(_) => break,
                                _ => {}
                            }
                        }
                        current_xaddr = text.trim().to_string();
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().local_name().into_inner();
                    if name == "Service" && in_service {
                        if !current_xaddr.is_empty() {
                            endpoints.push(OnvifServiceEndpoint {
                                namespace: current_ns.clone(),
                                xaddr: current_xaddr.clone(),
                            });
                        }
                        in_service = false;
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
        }
        endpoints
    }

    fn extract_capability_url(xml: &str, category: &str, _attr: &str) -> Option<String> {
        let bytes = xml.as_bytes();
        let mut reader = quick_xml::Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut in_category = false;
        let mut depth = 0;

        loop {
            use quick_xml::events::Event;
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = e.name().local_name().into_inner();
                    if name == category {
                        in_category = true;
                        depth = 1;
                        continue;
                    }
                    if in_category {
                        depth += 1;
                        if name == "XAddr" {
                            let mut text = String::new();
                            loop {
                                match reader.read_event() {
                                    Ok(Event::Text(ref e)) => text.push_str(&*e),
                                    Ok(Event::End(ref e)) => {
                                        let en = e.name().local_name().into_inner();
                                        if en == "XAddr" { break; }
                                    }
                                    Ok(Event::Eof) | Err(_) => break,
                                    _ => {}
                                }
                            }
                            if !text.trim().is_empty() {
                                return Some(text.trim().to_string());
                            }
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    if in_category {
                        depth -= 1;
                        if depth == 0 { in_category = false; }
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
        }

        None
    }

    /// 获取所有 Profile 的 RTSP 流地址
    pub async fn get_all_stream_uris(&self) -> anyhow::Result<Vec<(OnvifProfile, OnvifStreamUri)>> {
        let profiles = self.get_profiles().await?;
        let mut results = Vec::new();
        for profile in profiles {
            match self.get_stream_uri(&profile.token).await {
                Ok(uri) => results.push((profile, uri)),
                Err(e) => tracing::warn!("[ONVIF] Failed to get stream URI for profile {}: {}", profile.token, e),
            }
        }
        Ok(results)
    }

    /// 检查设备是否在线（通过 GetDeviceInformation）
    pub async fn is_online(&self) -> bool {
        self.get_device_info().await.is_ok()
    }
}