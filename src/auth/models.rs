use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(username: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self { id: Uuid::new_v4(), username, password_hash, created_at: now, updated_at: now }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Role {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self { id: Uuid::new_v4(), name, description, created_at: now, updated_at: now }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Permission {
    pub fn new(name: String, description: Option<String>) -> Self {
        Self { id: Uuid::new_v4(), name, description, created_at: Utc::now() }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest { pub username: String, pub password: String }

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String, pub refresh_token: String,
    pub expires_in: i64, pub token_type: String, pub user: UserInfo,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest { pub refresh_token: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String, pub password: String, pub role_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest { pub username: Option<String>, pub password: Option<String> }

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignRolesRequest { pub role_ids: Vec<Uuid> }

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String, pub description: Option<String>, pub permission_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct SetPermissionsRequest { pub permission_ids: Vec<Uuid> }

#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest { pub name: String, pub description: Option<String> }

#[derive(Debug, Serialize)]
pub struct UserWithRoles { #[serde(flatten)] pub user: User, pub roles: Vec<String> }

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: u16, pub message: String, pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self { Self { code: 200, message: "success".to_string(), data: Some(data) } }
    pub fn error(code: u16, message: &str) -> Self { Self { code, message: message.to_string(), data: None } }
}