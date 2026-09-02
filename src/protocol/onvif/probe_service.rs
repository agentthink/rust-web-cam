use crate::protocol::onvif::{
    OnvifDiscoveryClient, OnvifDeviceClient, OnvifDiscoveredDevice,
    OnvifDeviceInfo, OnvifCapabilities, OnvifProfile, OnvifStreamUri,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeResult {
    pub host: String,
    pub port: u16,
    pub urn: Option<String>,
    pub x_addr: String,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub types: Vec<String>,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub has_ptz: bool,
    #[serde(default)]
    pub has_streaming: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CapabilitiesResult {
    pub device_info: DeviceInfo,
    pub capabilities: CapabilityUrls,
    pub profiles: Vec<ProfileInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub serial_number: Option<String>,
    pub hardware_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CapabilityUrls {
    pub media: Option<String>,
    pub ptz: Option<String>,
    pub events: Option<String>,
    pub imaging: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProfileInfo {
    pub token: String,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamResult {
    pub streams: Vec<StreamInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamInfo {
    pub token: String,
    pub name: String,
    pub rtsp_url: String,
}

pub struct OnvifProbeService;

impl OnvifProbeService {
    pub async fn discover_multicast() -> anyhow::Result<Vec<ProbeResult>> {
        let devices = OnvifDiscoveryClient::discover().await?;
        Ok(devices.into_iter().map(|d| Self::to_probe_result(d)).collect())
    }

    pub async fn probe_unicast(ip: &str, port: u16) -> anyhow::Result<Option<ProbeResult>> {
        let device = OnvifDiscoveryClient::probe_unicast(ip, port).await?;
        Ok(device.map(|d| Self::to_probe_result(d)))
    }

    pub async fn get_capabilities(x_addr: &str, username: &str, password: &str) -> anyhow::Result<CapabilitiesResult> {
        let client = OnvifDeviceClient::new(x_addr)
            .with_credentials(username, password);

        let device_info = client.get_device_info().await?;
        let caps = client.get_capabilities_with_fallback().await?;
        let profiles = client.get_profiles().await?;

        Ok(CapabilitiesResult {
            device_info: DeviceInfo {
                manufacturer: device_info.manufacturer,
                model: device_info.model,
                firmware_version: device_info.firmware_version,
                serial_number: device_info.serial_number,
                hardware_id: device_info.hardware_id,
            },
            capabilities: CapabilityUrls {
                media: caps.media,
                ptz: caps.ptz,
                events: caps.events,
                imaging: caps.imaging,
            },
            profiles: profiles.into_iter().map(|p| ProfileInfo {
                token: p.token,
                name: p.name,
            }).collect(),
        })
    }

    pub async fn check_online(x_addr: &str, username: &str, password: &str) -> bool {
        let client = OnvifDeviceClient::new(x_addr)
            .with_credentials(username, password);
        client.is_online().await
    }

    pub async fn get_stream_uris(
        media_x_addr: Option<&str>,
        x_addr: &str,
        username: &str,
        password: &str,
        profile_tokens: &[String],
    ) -> anyhow::Result<StreamResult> {
        let url = media_x_addr.unwrap_or(x_addr);
        let client = OnvifDeviceClient::new(url)
            .with_credentials(username, password);

        let profiles = client.get_profiles().await?;
        let mut streams = Vec::new();
        for token in profile_tokens {
            match client.get_stream_uri(token).await {
                Ok(uri) => {
                    let profile_name = profiles
                        .iter()
                        .find(|p| p.token == *token)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| token.clone());

                    streams.push(StreamInfo {
                        token: token.clone(),
                        name: profile_name,
                        rtsp_url: uri.uri,
                    });
                }
                Err(e) => {
                    tracing::warn!("[ONVIF] Failed to get stream URI for {}: {}", token, e);
                }
            }
        }

        Ok(StreamResult { streams })
    }

    fn to_probe_result(d: OnvifDiscoveredDevice) -> ProbeResult {
        let has_ptz = d.types.iter().any(|t| t.contains("PTZ")) || d.scopes.iter().any(|s| s.contains("/PTZ"));
        let has_streaming = d.types.iter().any(|t| t.contains("Streaming")) || d.scopes.iter().any(|s| s.contains("/Streaming"));
        let (host, port) = Self::parse_x_addr(&d.x_addr);
        ProbeResult {
            host,
            port,
            urn: Some(d.address).filter(|s| !s.is_empty()),
            x_addr: d.x_addr,
            name: d.name.or_else(|| {
                d.manufacturer.as_ref().zip(d.model.as_ref())
                    .map(|(mfr, mdl)| format!("{} {}", mfr, mdl))
            }),
            manufacturer: d.manufacturer,
            model: d.model,
            types: d.types,
            scopes: d.scopes,
            has_ptz,
            has_streaming,
        }
    }

    fn parse_x_addr(x_addr: &str) -> (String, u16) {
        let is_https = x_addr.starts_with("https://");
        let url = x_addr.trim_start_matches("http://").trim_start_matches("https://");
        let parts: Vec<&str> = url.splitn(2, '/').collect();
        let host_part = parts.first().unwrap_or(&url);
        let mut segments = host_part.split(':');
        let host = segments.next().unwrap_or("").to_string();
        let default_port = if is_https { 443 } else { 80 };
        let port: u16 = segments.next().and_then(|p| p.parse().ok()).unwrap_or(default_port);
        (host, port)
    }
}
