use anyhow::{Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::debug;

pub struct MediaServerClient {
    base_url: String,
    secret: String,
    http: Client,
}

impl MediaServerClient {
    pub fn new(base_url: &str, secret: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        let base_url = base_url.trim_end_matches('/').to_string();
        Self { base_url, secret: secret.to_string(), http }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_with_params(path, &[]).await
    }

    pub async fn get_with_params<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &String)],
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut url_with_secret = format!("{}?secret={}", url, self.secret);
        for (k, v) in params {
            url_with_secret.push_str(&format!("&{}={}", k, v));
        }

        debug!("[MediaServer] GET {}", url_with_secret);

        let start = std::time::Instant::now();
        let resp = timeout(TokioDuration::from_secs(15), self.http.get(&url_with_secret).send()).await
            .map_err(|_| anyhow::anyhow!("GET request timeout after 15s"))??;
        let elapsed = start.elapsed();

        let status = resp.status();
        let body_text = resp.text().await
            .map_err(|e| anyhow::anyhow!("read body failed: {}", e))?;

        debug!("[MediaServer] GET {} -> {} ({}ms)", url, status, elapsed.as_millis());
        debug!("[MediaServer] GET {} response body: {}", url, body_text);

        if !status.is_success() {
            anyhow::bail!("API error: {} - {}", status, body_text);
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)
            .context("Deserialize response")?;
        serde_json::from_value(body).context("Deserialize response")
    }

    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self, path: &str, body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let url_with_secret = format!("{}?secret={}", url, self.secret);
        let body_json = serde_json::to_string(body).unwrap_or_default();

        debug!("[MediaServer] POST {}", url_with_secret);
        debug!("[MediaServer] POST {} body: {}", url, body_json);

        let start = std::time::Instant::now();
        let resp = timeout(TokioDuration::from_secs(15), self.http.post(&url_with_secret).json(body).send()).await
            .map_err(|_| anyhow::anyhow!("request timeout after 15s"))??;
        let elapsed = start.elapsed();

        let status = resp.status();
        let body_text = resp.text().await
            .map_err(|e| anyhow::anyhow!("read body failed: {}", e))?;

        debug!("[MediaServer] POST {} -> {} ({}ms)", url, status, elapsed.as_millis());
        debug!("[MediaServer] POST {} response body: {}", url, body_text);

        if !status.is_success() {
            anyhow::bail!("API error: {} - {}", status, body_text);
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)
            .context("Deserialize response")?;
        serde_json::from_value(body).context("Deserialize response")
    }

    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.post(path, &serde_json::Value::Null).await
    }

    pub fn base_url(&self) -> &str { &self.base_url }
}