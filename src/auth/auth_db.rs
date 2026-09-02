use rbatis::rbatis::RBatis;
use rbs::Value;
use rbdc_pg::PgDriver;
use async_trait::async_trait;
use std::sync::Arc;
use argon2::PasswordHasher;
use uuid::Uuid;
use crate::error::Result;
use super::models::{Permission, Role, User};
use super::repository::{UserRepository, RoleRepository, PermissionRepository};

pub struct PostgresAuthRepository {
    rb: RBatis,
}

impl PostgresAuthRepository {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let rb = RBatis::new();
        rb.init(PgDriver {}, database_url).ok();
        let repo = Self { rb };
        repo.init_tables().await;
        repo.seed_default_data().await;
        Ok(repo)
    }

    async fn init_tables(&self) {
        let tables = [
            "CREATE TABLE IF NOT EXISTS permissions (id UUID PRIMARY KEY, name VARCHAR(255) NOT NULL UNIQUE, description TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
            "CREATE TABLE IF NOT EXISTS roles (id UUID PRIMARY KEY, name VARCHAR(255) NOT NULL UNIQUE, description TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
            "CREATE TABLE IF NOT EXISTS users (id UUID PRIMARY KEY, username VARCHAR(255) NOT NULL UNIQUE, password_hash TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
            "CREATE TABLE IF NOT EXISTS user_roles (user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE, role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE, PRIMARY KEY (user_id, role_id))",
            "CREATE TABLE IF NOT EXISTS role_permissions (role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE, permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE, PRIMARY KEY (role_id, permission_id))",
        ];
        for sql in tables {
            if let Err(e) = self.rb.exec(sql, vec![]).await {
                tracing::warn!("[AuthDB] init_tables warning (may already exist): {}", e);
            }
        }
        tracing::info!("[AuthDB] init_tables completed");
    }

    async fn seed_default_data(&self) {
        let count: Vec<Value> = match self.rb.exec_decode("SELECT COUNT(*) as c FROM users WHERE username = 'admin'", vec![]).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("[AuthDB] seed_default_data skipped (table not ready): {}", e);
                return;
            }
        };
        if !count.is_empty() && count[0]["c"].as_i64().unwrap_or(0) > 0 { return; }

        let now = chrono::Utc::now().to_rfc3339();
        let admin_role_id = Uuid::new_v4();
        let admin_user_id = Uuid::new_v4();

        if let Err(e) = self.rb.exec("INSERT INTO roles (id, name, description, created_at, updated_at) VALUES ($1::uuid, $2, $3, $4, $5)",
                     vec![rbs::value![admin_role_id.to_string(), "Admin", "Administrator", now.clone(), now.clone()]]).await {
            tracing::warn!("[AuthDB] seed_default_data (role may exist): {}", e);
        }

        let salt = password_hash::phc::SaltString::generate();
        let hash = match argon2::Argon2::default().hash_password("admin123".as_bytes()) {
            Ok(h) => h.to_string(),
            Err(e) => {
                tracing::warn!("[AuthDB] seed_default_data (hash failed): {}", e);
                return;
            }
        };

        if let Err(e) = self.rb.exec("INSERT INTO users (id, username, password_hash, created_at, updated_at) VALUES ($1::uuid, $2, $3, $4, $5)",
                     vec![rbs::value![admin_user_id.to_string(), "admin", hash, now.clone(), now.clone()]]).await {
            tracing::warn!("[AuthDB] seed_default_data (user may exist): {}", e);
        }

        if let Err(e) = self.rb.exec("INSERT INTO user_roles (user_id, role_id) VALUES ($1::uuid, $2::uuid)",
                     vec![rbs::value![admin_user_id.to_string(), admin_role_id.to_string()]]).await {
            tracing::warn!("[AuthDB] seed_default_data (user_roles may exist): {}", e);
        }

        tracing::info!("[AuthDB] seed_default_data completed");
    }
}

impl Clone for PostgresAuthRepository {
    fn clone(&self) -> Self { Self { rb: self.rb.clone() } }
}

#[async_trait]
impl UserRepository for PostgresAuthRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let rows: Vec<Value> = self.rb.exec_decode(
            "SELECT id, username, password_hash, created_at, updated_at FROM users WHERE id = $1::uuid",
            vec![rbs::value![id.to_string()]],
        ).await?;
        Ok(rows.into_iter().next().map(|r| User {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            username: r["username"].as_str().unwrap_or("").to_string(),
            password_hash: r["password_hash"].as_str().unwrap_or("").to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let rows: Vec<Value> = self.rb.exec_decode(
            "SELECT id, username, password_hash, created_at, updated_at FROM users WHERE username = $1",
            vec![rbs::value![username]],
        ).await?;
        Ok(rows.into_iter().next().map(|r| User {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            username: r["username"].as_str().unwrap_or("").to_string(),
            password_hash: r["password_hash"].as_str().unwrap_or("").to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }))
    }

    async fn create(&self, user: &User) -> Result<()> {
        self.rb.exec("INSERT INTO users (id, username, password_hash, created_at, updated_at) VALUES ($1::uuid, $2, $3, $4, $5)",
                     vec![rbs::value![user.id.to_string(), user.username.clone(), user.password_hash.clone(), user.created_at.to_rfc3339(), user.updated_at.to_rfc3339()]]).await?;
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<()> {
        self.rb.exec("UPDATE users SET username = $1, password_hash = $2, updated_at = $3 WHERE id = $4::uuid",
                     vec![rbs::value![user.username.clone(), user.password_hash.clone(), user.updated_at.to_rfc3339(), user.id.to_string()]]).await?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        self.rb.exec("DELETE FROM users WHERE id = $1::uuid", vec![rbs::value![id.to_string()]]).await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<User>> {
        let rows: Vec<Value> = self.rb.exec_decode("SELECT id, username, password_hash, created_at, updated_at FROM users", vec![]).await?;
        Ok(rows.into_iter().map(|r| User {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            username: r["username"].as_str().unwrap_or("").to_string(),
            password_hash: r["password_hash"].as_str().unwrap_or("").to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }).collect())
    }

    async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<String>> {
        let rows: Vec<Value> = self.rb.exec_decode(
            "SELECT r.name FROM roles r INNER JOIN user_roles ur ON r.id::uuid = ur.role_id::uuid WHERE ur.user_id = $1::uuid",
            vec![rbs::value![user_id.to_string()]],
        ).await?;
        Ok(rows.into_iter().filter_map(|r| r["name"].as_str().map(String::from)).collect())
    }

    async fn set_user_roles(&self, user_id: Uuid, role_ids: &[Uuid]) -> Result<()> {
        self.rb.exec("DELETE FROM user_roles WHERE user_id = $1::uuid", vec![rbs::value![user_id.to_string()]]).await?;
        for role_id in role_ids {
            self.rb.exec("INSERT INTO user_roles (user_id, role_id) VALUES ($1::uuid, $2::uuid)",
                         vec![rbs::value![user_id.to_string(), role_id.to_string()]]).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl RoleRepository for PostgresAuthRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Role>> {
        let rows: Vec<Value> = self.rb.exec_decode("SELECT id, name, description, created_at, updated_at FROM roles WHERE id = $1::uuid",
                                                   vec![rbs::value![id.to_string()]]).await?;
        Ok(rows.into_iter().next().map(|r| Role {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            name: r["name"].as_str().unwrap_or("").to_string(),
            description: r["description"].as_str().map(String::from),
            created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
        }))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>> {
        let rows: Vec<Value> = self.rb.exec_decode("SELECT id, name, description, created_at, updated_at FROM roles WHERE name = $1",
                                                   vec![rbs::value![name]]).await?;
        Ok(rows.into_iter().next().map(|r| Role {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            name: r["name"].as_str().unwrap_or("").to_string(),
            description: r["description"].as_str().map(String::from),
            created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
        }))
    }

    async fn create(&self, role: &Role) -> Result<()> {
        self.rb.exec("INSERT INTO roles (id, name, description, created_at, updated_at) VALUES ($1::uuid, $2, $3, $4, $5)",
                     vec![rbs::value![role.id.to_string(), role.name.clone(), role.description.clone().unwrap_or_default(), role.created_at.to_rfc3339(), role.updated_at.to_rfc3339()]]).await?;
        Ok(())
    }

    async fn update(&self, role: &Role) -> Result<()> {
        self.rb.exec("UPDATE roles SET name = $1, description = $2, updated_at = $3 WHERE id = $4::uuid",
                     vec![rbs::value![role.name.clone(), role.description.clone().unwrap_or_default(), role.updated_at.to_rfc3339(), role.id.to_string()]]).await?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        self.rb.exec("DELETE FROM roles WHERE id = $1::uuid", vec![rbs::value![id.to_string()]]).await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Role>> {
        let rows: Vec<Value> = self.rb.exec_decode("SELECT id, name, description, created_at, updated_at FROM roles", vec![]).await?;
        Ok(rows.into_iter().map(|r| Role {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            name: r["name"].as_str().unwrap_or("").to_string(),
            description: r["description"].as_str().map(String::from),
            created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
        }).collect())
    }

    async fn get_role_permissions(&self, role_id: Uuid) -> Result<Vec<String>> {
        let rows: Vec<Value> = self.rb.exec_decode(
            "SELECT p.name FROM permissions p INNER JOIN role_permissions rp ON p.id::uuid = rp.permission_id::uuid WHERE rp.role_id = $1::uuid",
            vec![rbs::value![role_id.to_string()]]).await?;
        Ok(rows.into_iter().filter_map(|r| r["name"].as_str().map(String::from)).collect())
    }

    async fn set_role_permissions(&self, role_id: Uuid, permission_ids: &[Uuid]) -> Result<()> {
        self.rb.exec("DELETE FROM role_permissions WHERE role_id = $1::uuid", vec![rbs::value![role_id.to_string()]]).await?;
        for perm_id in permission_ids {
            self.rb.exec("INSERT INTO role_permissions (role_id, permission_id) VALUES ($1::uuid, $2::uuid)",
                         vec![rbs::value![role_id.to_string(), perm_id.to_string()]]).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl PermissionRepository for PostgresAuthRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Permission>> {
        let rows: Vec<Value> = self.rb.exec_decode("SELECT id, name, description, created_at FROM permissions WHERE id = $1::uuid",
                                                   vec![rbs::value![id.to_string()]]).await?;
        Ok(rows.into_iter().next().map(|r| Permission {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            name: r["name"].as_str().unwrap_or("").to_string(),
            description: r["description"].as_str().map(String::from),
            created_at: chrono::Utc::now(),
        }))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Permission>> {
        let rows: Vec<Value> = self.rb.exec_decode("SELECT id, name, description, created_at FROM permissions WHERE name = $1",
                                                   vec![rbs::value![name]]).await?;
        Ok(rows.into_iter().next().map(|r| Permission {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            name: r["name"].as_str().unwrap_or("").to_string(),
            description: r["description"].as_str().map(String::from),
            created_at: chrono::Utc::now(),
        }))
    }

    async fn create(&self, permission: &Permission) -> Result<()> {
        self.rb.exec("INSERT INTO permissions (id, name, description, created_at) VALUES ($1::uuid, $2, $3, $4)",
                     vec![rbs::value![permission.id.to_string(), permission.name.clone(), permission.description.clone().unwrap_or_default(), permission.created_at.to_rfc3339()]]).await?;
        Ok(())
    }

    async fn update(&self, permission: &Permission) -> Result<()> {
        self.rb.exec("UPDATE permissions SET name = $1, description = $2 WHERE id = $3::uuid",
                     vec![rbs::value![permission.name.clone(), permission.description.clone().unwrap_or_default(), permission.id.to_string()]]).await?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        self.rb.exec("DELETE FROM permissions WHERE id = $1::uuid", vec![rbs::value![id.to_string()]]).await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Permission>> {
        let rows: Vec<Value> = self.rb.exec_decode("SELECT id, name, description, created_at FROM permissions", vec![]).await?;
        Ok(rows.into_iter().map(|r| Permission {
            id: Uuid::parse_str(r["id"].as_str().unwrap_or("")).unwrap_or(Uuid::nil()),
            name: r["name"].as_str().unwrap_or("").to_string(),
            description: r["description"].as_str().map(String::from),
            created_at: chrono::Utc::now(),
        }).collect())
    }
}