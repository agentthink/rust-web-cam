use async_trait::async_trait;
use dashmap::DashMap;
use uuid::Uuid;

// ✅ 使用 auth 模块自己的错误类型，或使用 crate::error::Result
use crate::error::Result;  // = Result<T, AppError>

use super::models::{Permission, Role, User};

// ═══════════════════════════════════════════════════════════════
// Repository Traits
// ═══════════════════════════════════════════════════════════════

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn create(&self, user: &User) -> Result<()>;
    async fn update(&self, user: &User) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list(&self) -> Result<Vec<User>>;
    async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<String>>;
    async fn set_user_roles(&self, user_id: Uuid, role_ids: &[Uuid]) -> Result<()>;
}

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Role>>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>>;
    async fn create(&self, role: &Role) -> Result<()>;
    async fn update(&self, role: &Role) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list(&self) -> Result<Vec<Role>>;
    async fn get_role_permissions(&self, role_id: Uuid) -> Result<Vec<String>>;
    async fn set_role_permissions(&self, role_id: Uuid, permission_ids: &[Uuid]) -> Result<()>;
}

#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Permission>>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Permission>>;
    async fn create(&self, permission: &Permission) -> Result<()>;
    async fn update(&self, permission: &Permission) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list(&self) -> Result<Vec<Permission>>;
}

// ═══════════════════════════════════════════════════════════════
// In-Memory Implementation
// ═══════════════════════════════════════════════════════════════

/// 内存中的认证仓库实现（用于测试和开发）
pub struct InMemoryAuthRepository {
    users: DashMap<Uuid, User>,
    username_index: DashMap<String, Uuid>,
    roles: DashMap<Uuid, Role>,
    permissions: DashMap<Uuid, Permission>,
    user_roles: DashMap<Uuid, Vec<Uuid>>,
    role_permissions: DashMap<Uuid, Vec<Uuid>>,
}

impl Default for InMemoryAuthRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryAuthRepository {
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
            username_index: DashMap::new(),
            roles: DashMap::new(),
            permissions: DashMap::new(),
            user_roles: DashMap::new(),
            role_permissions: DashMap::new(),
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryAuthRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        Ok(self.users.get(&id).map(|u| u.clone()))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        Ok(self.username_index
            .get(username)
            .and_then(|id| self.users.get(&id).map(|u| u.clone())))
    }

    async fn create(&self, user: &User) -> Result<()> {
        self.users.insert(user.id, user.clone());
        self.username_index.insert(user.username.clone(), user.id);
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<()> {
        self.users.insert(user.id, user.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        if let Some((_, user)) = self.users.remove(&id) {
            self.username_index.remove(&user.username);
            self.user_roles.remove(&id);
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<User>> {
        Ok(self.users.iter().map(|u| u.clone()).collect())
    }

    async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<String>> {
        let role_ids = self.user_roles
            .get(&user_id)
            .map(|r| r.clone())
            .unwrap_or_default();

        let roles: Vec<String> = role_ids
            .iter()
            .filter_map(|id| self.roles.get(id).map(|r| r.name.clone()))
            .collect();

        Ok(roles)
    }

    async fn set_user_roles(&self, user_id: Uuid, role_ids: &[Uuid]) -> Result<()> {
        self.user_roles.insert(user_id, role_ids.to_vec());
        Ok(())
    }
}

#[async_trait]
impl RoleRepository for InMemoryAuthRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Role>> {
        Ok(self.roles.get(&id).map(|r| r.clone()))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>> {
        Ok(self.roles.iter().find(|r| r.name == name).map(|r| r.clone()))
    }

    async fn create(&self, role: &Role) -> Result<()> {
        self.roles.insert(role.id, role.clone());
        Ok(())
    }

    async fn update(&self, role: &Role) -> Result<()> {
        self.roles.insert(role.id, role.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        self.roles.remove(&id);
        self.role_permissions.remove(&id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Role>> {
        Ok(self.roles.iter().map(|r| r.clone()).collect())
    }

    async fn get_role_permissions(&self, role_id: Uuid) -> Result<Vec<String>> {
        let perm_ids = self.role_permissions
            .get(&role_id)
            .map(|p| p.clone())
            .unwrap_or_default();

        let perms: Vec<String> = perm_ids
            .iter()
            .filter_map(|id| self.permissions.get(id).map(|p| p.name.clone()))
            .collect();

        Ok(perms)
    }

    async fn set_role_permissions(&self, role_id: Uuid, permission_ids: &[Uuid]) -> Result<()> {
        self.role_permissions.insert(role_id, permission_ids.to_vec());
        Ok(())
    }
}

#[async_trait]
impl PermissionRepository for InMemoryAuthRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Permission>> {
        Ok(self.permissions.get(&id).map(|p| p.clone()))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Permission>> {
        Ok(self.permissions.iter().find(|p| p.name == name).map(|p| p.clone()))
    }

    async fn create(&self, permission: &Permission) -> Result<()> {
        self.permissions.insert(permission.id, permission.clone());
        Ok(())
    }

    async fn update(&self, permission: &Permission) -> Result<()> {
        self.permissions.insert(permission.id, permission.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        self.permissions.remove(&id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Permission>> {
        Ok(self.permissions.iter().map(|p| p.clone()).collect())
    }
}