use quick_xml::events::Event;
use quick_xml::Reader;
use crate::protocol::onvif::auth::UsernameToken;

/// ONVIF PTZ 客户端
///
/// 通过 SOAP 请求控制摄像头的云台操作。
///
/// # 创建方式
/// ```rust
/// // 方式1：从 GetCapabilities 获取真实 PTZ 地址
/// let client = OnvifPtzClient::new(ptz_url, Some("admin".into()), Some("123456".into()));
///
/// // 方式2：默认路径拼接
/// let client = OnvifPtzClient::from_host_port("192.168.1.100".into(), 8899, Some("admin".into()), Some("123456".into()));
/// ```
pub struct OnvifPtzClient {
    /// PTZ 服务完整地址（如 http://192.168.1.100:8899/onvif/ptz_service）
    ptz_url: String,
    /// ONVIF 用户名
    username: Option<String>,
    /// ONVIF 密码
    password: Option<String>,
    /// Profile Token（默认 Profile_1）
    profile_token: String,
    /// HTTP 客户端
    http_client: reqwest::Client,
}

impl OnvifPtzClient {
    /// 使用完整的 PTZ 地址创建客户端
    ///
    /// # 参数
    /// * `ptz_url` - PTZ 服务完整地址
    /// * `username` - 用户名
    /// * `password` - 密码
    pub fn new(
        ptz_url: String,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            ptz_url,
            username,
            password,
            profile_token: "Profile_1".to_string(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("ONVIF HTTP client build failed"),
        }
    }

    /// 从 IP 和端口创建客户端（使用默认路径 /onvif/ptz_service）
    ///
    /// # 参数
    /// * `host` - 摄像头 IP
    /// * `port` - ONVIF 端口
    /// * `username` - 用户名
    /// * `password` - 密码
    pub fn from_host_port(
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self::new(
            format!("http://{}:{}/onvif/ptz_service", host, port),
            username,
            password,
        )
    }

    /// 设置 Profile Token
    pub fn with_profile(mut self, profile_token: String) -> Self {
        self.profile_token = profile_token;
        self
    }

    // ═══════════════════════════════════════════════════════════
    // PTZ 控制方法
    // ═══════════════════════════════════════════════════════════

    /// 持续移动
    ///
    /// # 参数
    /// * `pan` - 水平速度 (-1.0 ~ 1.0，正=右，负=左)
    /// * `tilt` - 垂直速度 (-1.0 ~ 1.0，正=上，负=下)
    /// * `zoom` - 变焦速度 (-1.0 ~ 1.0，正=放大，负=缩小)
    pub async fn continuous_move(&self, pan: f64, tilt: f64, zoom: f64) -> anyhow::Result<()> {
        let body = format!(
            r#"<ContinuousMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
<Velocity>
<PanTilt x="{}" y="{}" xmlns="http://www.onvif.org/ver10/schema"/>
<Zoom x="{}" xmlns="http://www.onvif.org/ver10/schema"/>
</Velocity>
</ContinuousMove>"#,
            self.profile_token, pan, tilt, zoom
        );
        self.send_ptz_request(&body).await
    }

    /// 停止移动
    pub async fn stop(&self) -> anyhow::Result<()> {
        let body = format!(
            r#"<Stop xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
<PanTilt>true</PanTilt>
<Zoom>true</Zoom>
</Stop>"#,
            self.profile_token
        );
        self.send_ptz_request(&body).await
    }

    /// 转到预置位
    ///
    /// # 参数
    /// * `preset_token` - 预置位 Token
    pub async fn goto_preset(&self, preset_token: &str) -> anyhow::Result<()> {
        let body = format!(
            r#"<GotoPreset xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
<PresetToken>{}</PresetToken>
</GotoPreset>"#,
            self.profile_token, preset_token
        );
        self.send_ptz_request(&body).await
    }

    /// 设置预置位
    ///
    /// # 参数
    /// * `preset_token` - 预置位 Token（空字符串表示新建）
    /// * `preset_name` - 预置位名称
    ///
    /// # 返回
    /// 新创建的预置位 Token
    pub async fn set_preset(
        &self,
        preset_token: &str,
        preset_name: Option<&str>,
    ) -> anyhow::Result<String> {
        let name_xml = preset_name
            .map(|n| format!(r#"<PresetName>{}</PresetName>"#, n))
            .unwrap_or_default();
        let body = format!(
            r#"<SetPreset xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
<PresetToken>{}</PresetToken>
{}
</SetPreset>"#,
            self.profile_token, preset_token, name_xml
        );
        let resp = self.send_ptz_request_raw(&body).await?;
        self.parse_preset_token(&resp)
            .ok_or_else(|| anyhow::anyhow!("No preset token in response"))
    }

    /// 删除预置位
    ///
    /// # 参数
    /// * `preset_token` - 预置位 Token
    pub async fn remove_preset(&self, preset_token: &str) -> anyhow::Result<()> {
        let body = format!(
            r#"<RemovePreset xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
<PresetToken>{}</PresetToken>
</RemovePreset>"#,
            self.profile_token, preset_token
        );
        self.send_ptz_request(&body).await
    }

    /// 绝对移动
    ///
    /// # 参数
    /// * `pan` - 水平位置
    /// * `tilt` - 垂直位置
    /// * `zoom` - 变焦位置
    pub async fn absolute_move(&self, pan: f64, tilt: f64, zoom: f64) -> anyhow::Result<()> {
        let body = format!(
            r#"<AbsoluteMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
<Position>
<PanTilt x="{}" y="{}" xmlns="http://www.onvif.org/ver10/schema"/>
<Zoom x="{}" xmlns="http://www.onvif.org/ver10/schema"/>
</Position>
</AbsoluteMove>"#,
            self.profile_token, pan, tilt, zoom
        );
        self.send_ptz_request(&body).await
    }

    /// 相对移动
    ///
    /// # 参数
    /// * `pan` - 水平偏移
    /// * `tilt` - 垂直偏移
    /// * `zoom` - 变焦偏移
    pub async fn relative_move(&self, pan: f64, tilt: f64, zoom: f64) -> anyhow::Result<()> {
        let body = format!(
            r#"<RelativeMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
<Translation>
<PanTilt x="{}" y="{}" xmlns="http://www.onvif.org/ver10/schema"/>
<Zoom x="{}" xmlns="http://www.onvif.org/ver10/schema"/>
</Translation>
</RelativeMove>"#,
            self.profile_token, pan, tilt, zoom
        );
        self.send_ptz_request(&body).await
    }

    /// 获取 PTZ 状态
    ///
    /// # 返回
    /// 当前位置和移动状态
    pub async fn get_status(&self) -> anyhow::Result<crate::domain::ptz::PtzStatus> {
        let body = format!(
            r#"<GetStatus xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
</GetStatus>"#,
            self.profile_token
        );
        let resp = self.send_ptz_request_raw(&body).await?;
        self.parse_ptz_status(&resp)
    }

    /// 获取预置位列表
    pub async fn get_presets(&self) -> anyhow::Result<Vec<(String, Option<String>)>> {
        let body = format!(
            r#"<GetPresets xmlns="http://www.onvif.org/ver20/ptz/wsdl">
<ProfileToken>{}</ProfileToken>
</GetPresets>"#,
            self.profile_token
        );
        let resp = self.send_ptz_request_raw(&body).await?;
        Ok(self.parse_presets(&resp))
    }

    // ═══════════════════════════════════════════════════════════
    // 内部方法
    // ═══════════════════════════════════════════════════════════

    /// 构建带 WS-Security 认证的 SOAP 信封
    fn build_ws_username_token_envelope(&self, body: &str) -> String {
        let wsse_block = if let (Some(u), Some(p)) = (&self.username, &self.password) {
            let (nonce, created, digest) = UsernameToken::build_digest(u, p);
            format!(
                r#"<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" SOAP-ENV:mustUnderstand="1">
  <wsse:UsernameToken>
    <wsse:Username>{}</wsse:Username>
    <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{}</wsse:Password>
    <wsse:Nonce EncodingType="Base64Binary">{}</wsse:Nonce>
    <wsse:Created xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">{}</wsse:Created>
  </wsse:UsernameToken>
</wsse:Security>"#,
                u, digest, nonce, created
            )
        } else {
            String::new()
        };

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Header>{}</s:Header>
<s:Body>{}</s:Body>
</s:Envelope>"#,
            wsse_block, body
        )
    }

    /// 发送 PTZ 请求（不返回响应体）
    async fn send_ptz_request(&self, body: &str) -> anyhow::Result<()> {
        let envelope = self.build_ws_username_token_envelope(body);
        self.send_request(&self.ptz_url, &envelope).await
    }

    /// 发送 PTZ 请求（返回响应体）
    async fn send_ptz_request_raw(&self, body: &str) -> anyhow::Result<String> {
        let envelope = self.build_ws_username_token_envelope(body);
        self.send_request_raw(&self.ptz_url, &envelope).await
    }

    /// 发送 HTTP 请求（不返回响应体）
    async fn send_request(&self, url: &str, envelope: &str) -> anyhow::Result<()> {
        self.send_request_raw(url, envelope).await?;
        Ok(())
    }

    /// 发送 HTTP 请求（返回响应体）
    async fn send_request_raw(&self, url: &str, envelope: &str) -> anyhow::Result<String> {
        let resp = self
            .http_client
            .post(url)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(envelope.to_string())
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("ONVIF request failed: {} (status={})", url, resp.status());
        }

        Ok(resp.text().await?)
    }

    // ═══════════════════════════════════════════════════════════
    // XML 解析
    // ═══════════════════════════════════════════════════════════

    /// 解析 PTZ 状态响应
    fn parse_ptz_status(&self, body: &str) -> anyhow::Result<crate::domain::ptz::PtzStatus> {
        let mut reader = Reader::from_str(body);
        reader.config_mut().trim_text(true);

        let mut pan = None;
        let mut tilt = None;
        let mut zoom = None;
        let mut moving = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().local_name();

                    if name.into_inner() == "PanTilt" {
                        for attr in e.attributes().flatten() {
                            let key = attr.key.into_inner();
                            if let Ok(val) = attr.value.parse::<f64>() {
                                match key {
                                    "x" => pan = Some(val),
                                    "y" => tilt = Some(val),
                                    _ => {}
                                }
                            }
                        }
                    }
                    if name.into_inner() == "Zoom" {
                        for attr in e.attributes().flatten() {
                            let key = attr.key.into_inner();
                            if key == "x" {
                                zoom = attr.value.parse::<f64>().ok();
                            }
                        }
                    }
                    if name.into_inner() == "MoveStatus" {
                        for attr in e.attributes().flatten() {
                            if attr.value.as_ref() != "IDLE" {
                                moving = true;
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(crate::domain::ptz::PtzStatus {
            position_pan: pan,
            position_tilt: tilt,
            position_zoom: zoom,
            moving,
        })
    }

    /// 解析预置位 Token
    fn parse_preset_token(&self, body: &str) -> Option<String> {
        let mut reader = Reader::from_str(body);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().local_name();

                    if name.into_inner() == "Preset" || name.into_inner() == "PresetToken" {
                        for attr in e.attributes().flatten() {
                            let key = attr.key.into_inner();
                            if key == "token" {
                                return Some(attr.value.to_string());
                            }
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().local_name();
                    if name.into_inner() == "PresetToken" {
                        // 文本内容
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        None
    }

    fn parse_presets(&self, body: &str) -> Vec<(String, Option<String>)> {
        let mut results = Vec::new();
        let mut reader = Reader::from_str(body);
        reader.config_mut().trim_text(true);
        let mut current_token: Option<String> = None;
        let mut current_name: Option<String> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().local_name();
                    if name.into_inner() == "Preset" || name.into_inner() == "PTZPreset" {
                        current_token = None;
                        current_name = None;
                        for attr in e.attributes().flatten() {
                            let key = attr.key.into_inner();
                            if key == "token" {
                                current_token = Some(attr.value.to_string());
                            }
                        }
                    } else if name.into_inner() == "Name" {
                        // will capture text in next text event
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if current_token.is_some() && current_name.is_none() {
                        let text = (&*e).to_string();
                        if !text.is_empty() {
                            current_name = Some(text);
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().local_name();
                    if (name.into_inner() == "Preset" || name.into_inner() == "PTZPreset") && current_token.is_some() {
                        if let Some(token) = current_token.take() {
                            results.push((token, current_name.take()));
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        results
    }
}