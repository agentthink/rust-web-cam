use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("Username already exists")]
    UsernameExists,
    #[error("Role not found: {0}")]
    RoleNotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Database error: {0}")]
    Database(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AuthError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, 401, msg.clone()),
            AuthError::InvalidToken(msg) => (StatusCode::UNAUTHORIZED, 401, msg.clone()),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, 401, "Token expired".to_string()),
            AuthError::PermissionDenied => (StatusCode::FORBIDDEN, 403, "Permission denied".to_string()),
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, 401, "Invalid credentials".to_string()),
            AuthError::UserNotFound => (StatusCode::NOT_FOUND, 404, "User not found".to_string()),
            AuthError::UsernameExists => (StatusCode::BAD_REQUEST, 400, "Username already exists".to_string()),
            AuthError::RoleNotFound(msg) => (StatusCode::NOT_FOUND, 404, format!("Role not found: {}", msg)),
            AuthError::BadRequest(msg) => (StatusCode::BAD_REQUEST, 400, msg.clone()),
            AuthError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, 500, msg.clone()),
            AuthError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, 500, format!("Database error: {}", msg)),
        };

        let body = Json(serde_json::json!({ "code": code, "message": message, "data": null }));
        (status, body).into_response()
    }
}

impl From<crate::error::AppError> for AuthError {
    fn from(e: crate::error::AppError) -> Self { AuthError::Internal(e.to_string()) }
}

pub type Result<T> = std::result::Result<T, AuthError>;