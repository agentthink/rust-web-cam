use std::sync::Arc;
use casbin::{Enforcer, CoreApi, MgmtApi};
use tokio::sync::RwLock;
use super::errors::Result;
use super::repository::RoleRepository;

pub struct CasbinManager {
    enforcer: Arc<RwLock<Enforcer>>,
}

impl CasbinManager {
    pub async fn new<R: RoleRepository + 'static>(_role_repo: Arc<R>) -> Result<Self> {
        let enforcer = Enforcer::new("casbin_model.conf", "casbin_policy.csv")
            .await
            .map_err(|e| super::errors::AuthError::BadRequest(format!("Casbin init error: {}", e)))?;

        let enforcer = Arc::new(RwLock::new(enforcer));
        Ok(Self { enforcer })
    }

    pub async fn enforce(&self, role: &str, obj: &str, act: &str) -> bool {
        self.enforcer.read().await.enforce((role, obj, act)).unwrap_or(false)
    }
}