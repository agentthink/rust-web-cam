pub mod config;
pub mod errors;
pub mod jwt;
pub mod middleware;
pub mod casbin;
pub mod handlers;
pub mod models;
pub mod repository;
pub mod auth_db;
pub mod macros;

pub use config::JwtConfig;
pub use errors::{AuthError, Result};
pub use jwt::{decode_token, encode_token, generate_tokens, Claims};
pub use models::*;
pub use middleware::{jwt_auth_layer, CurrentUser, AuthState};
pub use casbin::CasbinManager;  // ✅ 添加这行