use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Device offline: {0}")]
    DeviceOffline(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Media server error: {0}")]
    MediaServerError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Auth error: {0}")]
    Auth(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, 404, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, 400, msg.clone()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, 500, msg.clone()),
            AppError::DeviceOffline(msg) => (StatusCode::SERVICE_UNAVAILABLE, 503, msg.clone()),
            AppError::SessionNotFound(msg) => (StatusCode::NOT_FOUND, 404, msg.clone()),
            AppError::MediaServerError(msg) => (StatusCode::BAD_GATEWAY, 502, msg.clone()),
            AppError::WebSocketError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, 500, msg.clone()),
            AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, 401, msg.clone()),
        };

        let body = Json(serde_json::json!({
            "code": code,
            "message": message,
            "data": serde_json::Value::Null
        }));

        (status, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<crate::auth::errors::AuthError> for AppError {
    fn from(err: crate::auth::errors::AuthError) -> Self {
        AppError::Auth(err.to_string())
    }
}

impl From<rbatis::rbdc::Error> for AppError {
    fn from(err: rbatis::rbdc::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;