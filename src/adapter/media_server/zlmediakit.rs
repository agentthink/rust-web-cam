use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::adapter::media_server::{MediaServerAdapter, Protocol, RecordingInfo, RecordingFile, RtpTransport, ServerStatus, StreamInfo, host_with_port_from_config, base_host_from_config, format_rtsp_auth};
use crate::adapter::media_server::client::MediaServerClient;
use crate::config::MediaServerConfig;
use crate::domain::device::PlayLinks;
use tracing::{info, error,debug, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;
pub struct ZlMediaKitAdapter {
    config: MediaServerConfig,
    client: MediaServerClient,
}

impl ZlMediaKitAdapter {
    pub fn new(config: MediaServerConfig) -> Self {
        let client = MediaServerClient::new(&config.url, &config.api_key);
        Self { config, client }
    }

    fn host_with_port(&self, protocol: Protocol) -> String {
        host_with_port_from_config(&self.config, protocol)
    }

    fn build_urls(&self, app: &str, stream_key: &str) -> (String, String, String, String) {
        let rtsp_host = self.host_with_port(Protocol::Rtsp);
        let rtmp_host = self.host_with_port(Protocol::Rtmp);
        let http_host = self.host_with_port(Protocol::Hls);
        let rtsp = format!("rtsp://{}/{}/{}", rtsp_host, app, stream_key);
        let rtmp = format!("rtmp://{}/{}/{}", rtmp_host, app, stream_key);
        let hls = format!("http://{}/{}/{}/hls.m3u8", http_host, app, stream_key);
        let webrtc = format!("http://{}/webrtc/play/{}/{}", http_host, app, stream_key);
        (rtsp, rtmp, hls, webrtc)
    }
}

#[async_trait]
impl MediaServerAdapter for ZlMediaKitAdapter {
    fn name(&self) -> &str { &self.config.name }
    fn tag(&self) -> &str { &self.config.server_tag }
    fn server_type(&self) -> &str { "zlmediakit" }

    async fn is_online(&self) -> bool {
        self.client.get::<serde_json::Value>("/index/api/getStatistic").await.is_ok()
    }

    async fn get_status(&self) -> anyhow::Result<ServerStatus> {
        #[derive(Deserialize)]
        struct ApiResp { code: i32, data: Option<serde_json::Value> }
        let resp: ApiResp = self.client.get("/index/api/getStatistic").await
            .map_err(|e| anyhow::anyhow!("ZLMediaKit getStatistic failed: {} (url: {}{})", e, self.client.base_url(), "/index/api/getStatistic"))?;
        let data = resp.data.unwrap_or_default();

        Ok(ServerStatus {
            name: self.config.name.clone(),
            server_type: "zlmediakit".to_string(),
            online: true,
            session_count: data.get("totalTcp").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            cpu_usage: data.get("cpu").and_then(|v| v.as_f64()).unwrap_or(0.0),
            memory_usage: data.get("mem").and_then(|v| v.as_f64()).unwrap_or(0.0),
            bandwidth_in: 0, bandwidth_out: 0, last_heartbeat: None,
        })
    }

    async fn add_stream_proxy(&self, app: &str, stream_key: &str, rtsp_url: &str) -> anyhow::Result<StreamInfo> {
        let vhost = "__defaultVhost__".to_string();
        let app_str = app.to_string();
        let stream_str = stream_key.to_string();
        let url_str = rtsp_url.to_string();
        let rtp_type = "0".to_string();
        let timeout_ms = "10000".to_string();

        let params = vec![
            ("vhost", &vhost),
            ("app", &app_str),
            ("stream", &stream_str),
            ("url", &url_str),
            ("rtp_type", &rtp_type),
            ("timeout_ms", &timeout_ms),
        ];

        let ret_data: serde_json::Value = self.client.get_with_params("/index/api/addStreamProxy", &params).await?;
        tracing::debug!("/index/api/addStreamProxy return = {}" , ret_data.to_string());

        if let Some(code_val) = ret_data.get("code") {
            let code = code_val.as_i64().unwrap_or(0);
            if code == -1 {
                let msg = ret_data.get("msg").and_then(|v| v.as_str()).unwrap_or("");
                if msg.contains("already exists") || msg.contains("该流已经存在") {
                    tracing::info!("[ZlMediaKit] Stream {} already exists, treating as success", stream_key);
                } else {
                    anyhow::bail!("ZLMediaKit addStreamProxy failed: code={}, msg={}", code, msg);
                }
            } else if code != 0 {
                let msg = ret_data.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown");
                anyhow::bail!("ZLMediaKit addStreamProxy failed: code={}, msg={}", code, msg);
            }
        }

        let (rtsp, rtmp, hls, webrtc) = self.build_urls(app, stream_key);
        Ok(StreamInfo {
            stream_key: stream_key.to_string(),
            play_url: format!("http://{}/{}/{}/hls.m3u8", self.host_with_port(Protocol::Hls),app, stream_key),
            rtsp_url: rtsp, rtmp_url: rtmp, hls_url: hls, webrtc_url: webrtc,
            flv_url: None, web_flv_url: None,
            media_server_id: self.config.name.clone(),
            media_server_name: self.config.name.clone(),
        })
    }

    async fn remove_stream_proxy(&self, app: &str, stream_key: &str) -> anyhow::Result<()> {
        #[derive(Serialize)]
        struct CloseReq { schema: String, vhost: String, app: String, stream: String, force: bool }
        let req = CloseReq {
            schema: "rtsp".to_string(),
            vhost: "__defaultVhost__".to_string(),
            app: app.to_string(),
            stream: stream_key.to_string(),
            force: true
        };

        match self.client.post::<serde_json::Value, _>("/index/api/closeStreams", &req).await {
            Ok(resp) => {
                tracing::debug!("[ZLMediaKit] closeStreams response: {:?}", resp);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("[ZLMediaKit] closeStreams failed for {}/{}: {}", app, stream_key, e);
                Ok(())
            }
        }
    }

    async fn get_play_url(&self, app: &str, stream_key: &str, protocol: Protocol) -> anyhow::Result<String> {
        let host = self.host_with_port(protocol);
        match protocol {
            Protocol::Rtsp => Ok(format!("rtsp://{}/{}/{}", host, app, stream_key)),
            Protocol::Rtmp => Ok(format!("rtmp://{}/{}/{}", host, app, stream_key)),
            Protocol::Hls => Ok(format!("http://{}/{}/{}/hls.m3u8", host, app, stream_key)),
            Protocol::Http => Ok(format!("http://{}/{}/", host, app)),
            Protocol::WebRTC => Ok(format!("http://{}/webrtc/play/{}/{}", host, app, stream_key)),
            Protocol::Flv => Ok(format!("http://{}/{}/{}.live.flv", host, app, stream_key)),
            Protocol::WsFlv => Ok(format!("ws://{}/{}/{}.live.flv", host, app, stream_key)),
        }
    }

    async fn get_session_count(&self) -> anyhow::Result<u32> {
        #[derive(Deserialize)]
        struct Resp { code: i32, data: Option<Vec<serde_json::Value>> }
        let resp: Resp = self.client.get("/index/api/getAllSession").await?;
        Ok(resp.data.map(|v| v.len() as u32).unwrap_or(0))
    }

    async fn get_sessions(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        #[derive(Deserialize)]
        struct Resp { code: i32, data: Option<Vec<serde_json::Value>> }
        let resp: Resp = self.client.get("/index/api/getAllSession").await?;
        Ok(resp.data.unwrap_or_default())
    }

    async fn is_stream_online(&self, app: &str, stream_key: &str) -> anyhow::Result<bool> {
        let params = [
            ("vhost", &"__defaultVhost__".to_string()),
            ("app", &app.to_string()),
            ("stream", &stream_key.to_string()),
        ];

        tracing::info!("[ZLMediaKit] is_stream_online: app={}, stream={}", app, stream_key);

        #[derive(Deserialize)]
        struct Resp { code: i32, data: Option<Vec<serde_json::Value>>, msg: Option<String> }
        let resp: Resp = self.client.get_with_params("/index/api/getMediaList", &params).await?;

        tracing::info!("[ZLMediaKit] is_stream_online: app={}, stream={} -> code={}, msg={:?}, data_count={}", 
            app, stream_key, resp.code, resp.msg, resp.data.as_ref().map(|d| d.len()).unwrap_or(0));

        // 检查返回的流是否匹配我们查询的 stream_key
        if let Some(data) = &resp.data {
            for item in data {
                if let Some(item_stream) = item.get("stream").and_then(|v| v.as_str()) {
                    tracing::info!("[ZLMediaKit] is_stream_online: found stream in response: stream={}", item_stream);
                }
            }
        }

        Ok(resp.code == 0 && resp.data.as_ref().map(|d| !d.is_empty()).unwrap_or(false))
    }

    async fn ptz_control(&self, _stream_key: &str, _command: &str, _channel: u8) -> anyhow::Result<()> { Ok(()) }
    async fn start_recording(&self, app: &str, stream_key: &str, format: &str, output_path: Option<&str>) -> anyhow::Result<RecordingInfo> {
        let record_type = match format.to_lowercase().as_str() {
            "hls" | "flv" | "ts" => 0,
            _ => 1, // MP4
        };

        #[derive(Deserialize)]
        struct ApiResp { code: i32, msg: Option<String> }

        #[derive(Serialize)]
        struct Req<'a> {
            vhost: &'a str,
            app: &'a str,
            stream: &'a str,
            r#type: u8,
        }
        let req = Req {
            vhost: "__defaultVhost__",
            app,
            stream: stream_key,
            r#type: record_type,
        };
        let resp: ApiResp = self.client.post("/index/api/startRecord", &req).await?;
        if resp.code != 0 {
            return Err(anyhow::anyhow!("ZLMediaKit startRecord failed: {}", resp.msg.unwrap_or_default()));
        }

        tracing::info!(
            "[ZlMediaKit] Recording started: app={}, stream={}, format={}",
            app, stream_key, format
        );

        Ok(RecordingInfo {
            stream_key: stream_key.to_string(),
            output_path: output_path.unwrap_or("").to_string(),
            started_at: chrono::Utc::now().timestamp(),
        })
    }

    async fn stop_recording(&self, app: &str, stream_key: &str, format: &str) -> anyhow::Result<()> {
        let record_type = match format.to_lowercase().as_str() {
            "hls" | "flv" | "ts" => 0,
            _ => 1, // MP4
        };

        #[derive(Deserialize)]
        struct ApiResp { code: i32, msg: Option<String> }
        #[derive(Serialize)]
        struct Req<'a> {
            vhost: &'a str,
            app: &'a str,
            stream: &'a str,
            r#type: u8,
        }
        let req = Req {
            vhost: "__defaultVhost__",
            app,
            stream: stream_key,
            r#type: record_type,
        };
        let resp: ApiResp = self.client.post("/index/api/stopRecord", &req).await?;
        if resp.code != 0 {
            return Err(anyhow::anyhow!("ZLMediaKit stopRecord failed: {}", resp.msg.unwrap_or_default()));
        }

        tracing::info!(
            "[ZlMediaKit] Recording stopped: app={}, stream={}, format={}",
            app, stream_key, format
        );

        Ok(())
    }
    async fn is_recording(&self, _stream_key: &str, _format: &str) -> anyhow::Result<bool> { Ok(false) }
    async fn list_recordings(&self, app: &str, stream_key: &str) -> anyhow::Result<Vec<RecordingFile>> {
        #[derive(Deserialize)]
        struct Resp {
            code: i32,
            data: Option<Vec<FileEntry>>,
        }
        #[derive(Deserialize)]
        struct FileEntry {
            start_time: Option<f64>,
            file_path: Option<String>,
            file_size: Option<u64>,
            time_len: Option<f64>,
        }

        let zero = "0".to_string();
        let max_time = "9999999999".to_string();
        let app_str = app.to_string();
        let stream_str = stream_key.to_string();
        let vhost = "__defaultVhost__".to_string();
        let params: Vec<(&str, &String)> = vec![
            ("vhost", &vhost),
            ("app", &app_str),
            ("stream", &stream_str),
            ("start_time", &zero),
            ("end_time", &max_time),
        ];
        let resp: Resp = self.client.get_with_params("/index/api/getRecordFileList", &params).await?;
        if resp.code != 0 {
            return Ok(vec![]);
        }
        let files = resp.data.unwrap_or_default();
        Ok(files
            .into_iter()
            .filter_map(|f| {
                let path = f.file_path?;
                let filename = std::path::Path::new(&path)
                    .file_name()?
                    .to_str()?
                    .to_string();
                Some(RecordingFile {
                    filename,
                    path,
                    size: f.file_size.unwrap_or(0),
                    duration_secs: f.time_len.unwrap_or(0.0) as u64,
                    created_at: f.start_time.unwrap_or(0.0) as i64,
                    stream_key: Some(stream_key.to_string()),
                    media_server_name: Some(self.config.name.clone()),
                })
            })
            .collect())
    }

    async fn open_rtp_server(&self, stream_id: &str, port: u16, transport: RtpTransport) -> anyhow::Result<(u16, String)> {
        let enable_tcp = match transport { RtpTransport::Udp => 0, _ => 1 };
        let tcp_mode = match transport { RtpTransport::TcpPassive => 1, _ => 0 };

        #[derive(Deserialize)]
        struct Resp { code: i32, port: Option<u16> }
        let params = [
            ("port", &port.to_string()),
            ("enable_tcp", &enable_tcp.to_string()),
            ("tcp_mode", &tcp_mode.to_string()),
            ("stream_id", &stream_id.to_string()),
        ];
        let resp: Resp = self.client.get_with_params("/index/api/openRtpServer", &params).await?;
        Ok((resp.port.unwrap_or(port), base_host_from_config(&self.config)))
    }

    async fn close_rtp_server(&self, stream_id: &str) -> anyhow::Result<()> {
        let params = [("stream_id", &stream_id.to_string())];
        let _: serde_json::Value = self.client.get_with_params("/index/api/closeRtpServer", &params).await?;
        Ok(())
    }

    async fn get_media_info(&self, app: &str, stream_key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let vhost = "__defaultVhost__".to_string();
        let schema = "rtsp".to_string();
        let app_str = app.to_string();
        let stream_str = stream_key.to_string();
        let params: Vec<(&str, &String)> = vec![
            ("vhost", &vhost),
            ("app", &app_str),
            ("stream", &stream_str),
            ("schema", &schema),
        ];
        #[derive(Deserialize)]
        struct Resp { code: i32, tracks: Option<Vec<serde_json::Value>> }
        let resp: Resp = self.client.get_with_params("/index/api/getMediaInfo", &params).await?;
        Ok(resp.tracks.map(|t| serde_json::json!({ "tracks": t })))
    }

    async fn build_play_links(
        &self,
        app: &str,
        stream_key: &str,
        token: &str,
        expires_at: i64,
        rtsp_auth: Option<(&str, &str)>,
    ) -> anyhow::Result<PlayLinks> {
        tracing::info!("[ZLMediaKit] config.url={}, http_flv={:?}, hls={:?}", self.config.url, self.config.protocol_ports.http_flv, self.config.protocol_ports.hls);
        let rtsp_signaling_host = self.host_with_port(Protocol::Rtsp);
        let rtsp_media_host = self.host_with_port(Protocol::Rtsp);
        let hls_host = self.host_with_port(Protocol::Hls);
        let flv_host = self.host_with_port(Protocol::Flv);
        let ws_flv_host = self.host_with_port(Protocol::WsFlv);
        let webrtc_host = self.host_with_port(Protocol::WebRTC);

        tracing::info!("[ZLMediaKit] hosts: rtsp={}, hls={}, flv={}, ws_flv={}, webrtc={}", rtsp_signaling_host, hls_host, flv_host, ws_flv_host, webrtc_host);

        let rtsp_auth_str = format_rtsp_auth(rtsp_auth);

        Ok(PlayLinks {
            token: token.to_string(),
            stream_id: stream_key.to_string(),
            expires_at,
            ports: self.config.protocol_ports.clone(),
            rtsp_signaling: Some(format!(
                "rtsp://{}{}/{}/{}?token={}",
                rtsp_auth_str, rtsp_signaling_host,
                app, stream_key, token
            )),
            rtsp_media: Some(format!(
                "rtsp://{}{}/{}/{}?token={}",
                rtsp_auth_str, rtsp_media_host,
                app, stream_key, token
            )),
            flv: Some(format!(
                "http://{}/{}/{}.live.flv?token={}",
                flv_host,
                app, stream_key, token
            )),
            hls: Some(format!(
                "http://{}/{}/{}/hls.m3u8?token={}",
                hls_host,
                app, stream_key, token
            )),
            webrtc: Some(format!(
                "http://{}/webrtc/play?app={}&stream={}&type=play",
                webrtc_host,
                app, stream_key
            )),
            web_flv: Some(format!(
                "ws://{}/{}/{}.live.flv?token={}",
                ws_flv_host,
                app, stream_key, token
            )),
        })
    }
}