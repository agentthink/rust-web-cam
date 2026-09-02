use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expire_minutes: i64,
    pub refresh_token_expire_days: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
            access_token_expire_minutes: 15,
            refresh_token_expire_days: 7,
        }
    }
}