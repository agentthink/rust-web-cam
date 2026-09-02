use super::{config::JwtConfig, errors::AuthError, models::UserInfo};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub roles: Vec<String>,
    pub exp: i64,
    pub iat: i64,
    pub token_type: String,
}

impl Claims {
    pub fn new(
        user_id: Uuid,
        username: &str,
        roles: Vec<String>,
        token_type: &str,
        expires_in: Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            roles,
            exp: (now + expires_in).timestamp(),
            iat: now.timestamp(),
            token_type: token_type.to_string(),
        }
    }
}

pub fn encode_token(
    user_id: Uuid,
    username: &str,
    roles: Vec<String>,
    token_type: &str,
    expires_in: Duration,
    secret: &str,
) -> Result<String, AuthError> {
    let claims = Claims::new(user_id, username, roles, token_type, expires_in);
    let header = Header::new(Algorithm::HS256);

    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::InvalidToken(e.to_string()))
}

pub fn decode_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| {
        if e.to_string().contains("ExpiredSignature") {
            AuthError::TokenExpired
        } else {
            AuthError::InvalidToken(e.to_string())
        }
    })
}

pub fn generate_tokens(user: &UserInfo, config: &JwtConfig) -> Result<(String, String), AuthError> {
    tracing::debug!(
        "[JWT] Generating tokens for user: {} (id={}), roles: {:?}",
        user.username,
        user.id,
        user.roles
    );
    tracing::debug!(
        "[JWT] Access token expires in: {} minutes, Refresh token expires in: {} days",
        config.access_token_expire_minutes,
        config.refresh_token_expire_days
    );

    let access_token = encode_token(
        user.id,
        &user.username,
        user.roles.clone(),
        "access",
        Duration::minutes(config.access_token_expire_minutes),
        &config.secret,
    )?;

    let refresh_token = encode_token(
        user.id,
        &user.username,
        user.roles.clone(),
        "refresh",
        Duration::days(config.refresh_token_expire_days),
        &config.secret,
    )?;

    tracing::debug!(
        "[JWT] Tokens generated: access_len={}, refresh_len={}",
        access_token.len(),
        refresh_token.len()
    );

    Ok((access_token, refresh_token))
}
