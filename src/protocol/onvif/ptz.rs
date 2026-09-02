use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PtzDirection { Up, Down, Left, Right, ZoomIn, ZoomOut }

pub struct OnvifPtzService {
    url: String,
    username: Option<String>,
    password: Option<String>,
}

impl OnvifPtzService {
    pub fn new(url: String, username: Option<String>, password: Option<String>) -> Self {
        Self { url, username, password }
    }

    pub fn into_client(self) -> crate::protocol::onvif::ptz_client::OnvifPtzClient {
        crate::protocol::onvif::ptz_client::OnvifPtzClient::new(self.url, self.username, self.password)
    }
}