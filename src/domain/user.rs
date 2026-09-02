use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum UserRole {
    Admin,
    Operator,
    User,
    Guest,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(username: String, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            password_hash,
            role: UserRole::default(),
            created_at: Utc::now(),
        }
    }

    pub fn with_role(mut self, role: UserRole) -> Self {
        self.role = role;
        self
    }

    pub fn is_admin(&self) -> bool { matches!(self.role, UserRole::Admin) }
    pub fn is_operator(&self) -> bool { matches!(self.role, UserRole::Operator | UserRole::Admin) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Option<UserRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
}