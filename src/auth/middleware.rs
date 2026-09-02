use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use uuid::Uuid;
use super::{
    auth_db::PostgresAuthRepository, casbin::CasbinManager,
    config::JwtConfig, errors::AuthError, jwt::decode_token, models::UserInfo,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct CurrentUser(pub UserInfo);

#[derive(Clone)]
pub struct AuthState {
    pub jwt_config: JwtConfig,
    pub casbin: Arc<CasbinManager>,
    pub repo: Arc<PostgresAuthRepository>,
}

impl AuthState {
    pub fn new(jwt_config: JwtConfig, casbin: Arc<CasbinManager>, repo: Arc<PostgresAuthRepository>) -> Self {
        Self { jwt_config, casbin, repo }
    }
}

pub async fn jwt_auth_layer(
    State(auth_state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Response {
    let auth_header = req.headers().get("Authorization")
        .and_then(|v| v.to_str().ok());

    let auth_header = match auth_header {
        Some(h) if h.starts_with("Bearer ") => h,
        _ => return AuthError::Unauthorized("Missing or invalid Authorization header".to_string()).into_response(),
    };

    let token = &auth_header[7..];
    let claims = match decode_token(token, &auth_state.jwt_config.secret) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    if claims.token_type != "access" {
        return AuthError::InvalidToken("Invalid token type".to_string()).into_response();
    }

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return AuthError::InvalidToken("Invalid user ID".to_string()).into_response(),
    };

    let user_info = UserInfo { id: user_id, username: claims.username, roles: claims.roles };
    req.extensions_mut().insert(CurrentUser(user_info));
    next.run(req).await
}