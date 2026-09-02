use std::sync::Arc;
use dashmap::DashMap;
use crate::adapter::media_server::MediaServerAdapter;
use crate::config::MediaServerConfig;
use crate::domain::server::{Server, ServerProtocolPorts, ServerType};
use crate::infrastructure::DbRepository;
use crate::infrastructure::cluster::ClusterManager;

/// 托管的媒体服务器
#[derive(Debug, Clone)]
pub struct ManagedServer {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub server_type: ServerType,
    pub weight: u32,
    pub enabled: bool,
    pub server_tag: String,
    pub protocol_ports: ServerProtocolPorts,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ManagedServer {
    pub fn new(
        name: String,
        url: String,
        api_key: String,
        server_type: ServerType,
        weight: u32,
        server_tag: String,
        protocol_ports: ServerProtocolPorts,
    ) -> Self {
        Self {
            id: 0,
            name,
            url,
            api_key,
            server_type,
            weight,
            enabled: true,
            server_tag,
            protocol_ports,
            created_at: chrono::Utc::now(),
        }
    }

    /// 转换为领域模型
    pub fn to_domain(&self) -> Server {
        Server {
            id: self.id,
            name: self.name.clone(),
            url: self.url.clone(),
            api_key: self.api_key.clone(),
            server_type: self.server_type,
            weight: self.weight,
            enabled: self.enabled,
            server_tag: self.server_tag.clone(),
            protocol_ports: self.protocol_ports.clone(),
            created_at: self.created_at,
            updated_at: chrono::Utc::now(),
        }
    }

    /// 从领域模型创建
    pub fn from_domain(s: Server) -> Self {
        Self {
            id: s.id,
            name: s.name,
            url: s.url,
            api_key: s.api_key,
            server_type: s.server_type,
            weight: s.weight,
            enabled: s.enabled,
            server_tag: s.server_tag,
            protocol_ports: s.protocol_ports,
            created_at: s.created_at,
        }
    }

    /// 转换为媒体服务器配置
    pub fn to_config(&self) -> MediaServerConfig {
        MediaServerConfig {
            name: self.name.clone(),
            url: self.url.clone(),
            api_key: self.api_key.clone(),
            server_type: self.server_type.to_string(),
            weight: self.weight,
            enabled: self.enabled,
            server_tag: self.server_tag.clone(),
            protocol_ports: self.protocol_ports.clone(),
        }
    }
}

/// 媒体服务器管理服务
///
/// 负责媒体服务器的注册、更新、删除和查询。
/// 使用 DashMap 维护内存中的服务器列表，并持久化到数据库。
///
/// # 依赖
/// - DbRepository: 数据库持久化
pub struct MediaServerService {
    /// 服务器列表（内存缓存）
    servers: Arc<DashMap<String, ManagedServer>>,
    /// 数据库仓库
    repo: Arc<DbRepository>,
    /// 集群管理器（自动注册/注销在线服务器）
    cluster: Arc<ClusterManager>,
}

impl MediaServerService {
    /// 创建媒体服务器管理服务
    ///
    /// # 参数
    /// * `repo` - 数据库仓库
    /// * `cluster` - 集群管理器
    pub fn new(repo: Arc<DbRepository>, cluster: Arc<ClusterManager>) -> Self {
        let servers = Arc::new(DashMap::new());

        // 从数据库加载已有服务器
        let db_count = repo.servers_cache().len();
        tracing::info!("[MediaServerService] Loading {} servers from DB cache", db_count);

        for db_server in repo.servers_cache().iter().map(|r| r.value().clone()) {
            let tag = if db_server.server_tag.is_empty() {
                db_server.name.clone()
            } else {
                db_server.server_tag.clone()
            };

            let server_type = ServerType::from_str(&db_server.server_type)
                .unwrap_or(ServerType::Zlmediakit);

            let ms = ManagedServer {
                id: db_server.id,
                name: db_server.name,
                url: db_server.url,
                api_key: db_server.api_key,
                server_type,
                weight: db_server.weight as u32,
                enabled: db_server.enabled,
                server_tag: tag,
                protocol_ports: serde_json::from_value(db_server.protocol_ports.clone())
                    .unwrap_or_default(),
                created_at: chrono::DateTime::from_timestamp(
                    db_server.created_at.unix_timestamp(), 0,
                )
                    .unwrap_or_else(chrono::Utc::now),
            };

            tracing::info!(
                "[MediaServerService]   DB server: tag={}, name={}",
                ms.server_tag, ms.name
            );
            servers.insert(ms.server_tag.clone(), ms.clone());
            
            // 注册数据库服务器到集群
            if ms.enabled {
                cluster.register_server(ms.to_config());
                tracing::info!("[MediaServerService]   DB server {} registered to cluster", ms.server_tag);
            }
        }

        tracing::info!(
            "[MediaServerService] Total servers loaded: {}",
            servers.len()
        );

        Self { servers, repo, cluster }
    }

    /// 获取所有服务器列表
    pub fn list(&self) -> Vec<ManagedServer> {
        self.servers.iter().map(|r| r.value().clone()).collect()
    }

    /// 根据标签获取服务器
    pub fn get(&self, tag: &str) -> Option<ManagedServer> {
        self.servers.get(tag).map(|r| r.value().clone())
    }

    /// 根据名称获取服务器
    pub fn get_by_name(&self, name: &str) -> Option<ManagedServer> {
        self.servers
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.value().clone())
    }

    /// 插入服务器（仅内存）
    pub fn insert(&self, ms: ManagedServer) {
        self.servers.insert(ms.server_tag.clone(), ms);
    }

    /// 创建服务器（持久化到数据库）
    ///
    /// # 参数
    /// * `name` - 服务器名称
    /// * `url` - API 地址
    /// * `api_key` - API 密钥
    /// * `server_type` - 服务器类型
    /// * `weight` - 权重
    /// * `server_tag` - 唯一标签
    /// * `protocol_ports` - 协议端口配置
    ///
    /// # 返回
    /// 创建成功的 ManagedServer（包含数据库分配的 ID）
    pub async fn create(
        &self,
        name: String,
        url: String,
        api_key: String,
        server_type: ServerType,
        weight: u32,
        server_tag: String,
        protocol_ports: ServerProtocolPorts,
    ) -> ManagedServer {
        let mut ms = ManagedServer::new(
            name, url, api_key, server_type, weight, server_tag, protocol_ports,
        );

        // 持久化到数据库
        let domain = ms.to_domain();
        match self.repo.create_server(&domain).await {
            Ok(id) => {
                ms.id = id;
                tracing::info!(
                    "[MediaServerService] Created server: id={}, tag={}",
                    id, ms.server_tag
                );
            }
            Err(e) => {
                tracing::warn!(
                    "[MediaServerService] Failed to persist server to DB: {}",
                    e
                );
            }
        }

        self.servers.insert(ms.server_tag.clone(), ms.clone());
        
        // 注册到集群（新增服务器默认为在线）
        if ms.enabled {
            self.cluster.register_server(ms.to_config());
            tracing::info!("[MediaServerService] Server {} registered to cluster", ms.server_tag);
        }
        
        ms
    }

    /// 更新服务器
    pub async fn update(
        &self,
        tag: &str,
        name: String,
        url: String,
        api_key: String,
        server_type: ServerType,
        weight: u32,
        server_tag: String,
        protocol_ports: ServerProtocolPorts,
    ) -> Option<ManagedServer> {
        if let Some(mut entry) = self.servers.get_mut(tag) {
            // 更新字段
            let old_tag = entry.server_tag.clone();
            entry.name = name;
            entry.url = url;
            entry.api_key = api_key;
            entry.server_type = server_type;
            entry.weight = weight;
            entry.server_tag = server_tag;
            entry.protocol_ports = protocol_ports;

            let updated = entry.value().clone();
            drop(entry);

            // 如果标签变了，需要重新插入
            if updated.server_tag != old_tag {
                self.servers.remove(&old_tag);
                self.servers.insert(updated.server_tag.clone(), updated.clone());
            }

            // 持久化
            let domain = updated.to_domain();
            if let Err(e) = self.repo.update_server(&domain).await {
                tracing::warn!(
                    "[MediaServerService] Failed to persist server update: {}",
                    e
                );
            }

            // 重新注册到集群（更新 adapter 的配置缓存）
            self.cluster.unregister_server(&updated.server_tag);
            if updated.enabled {
                self.cluster.register_server(updated.to_config());
                tracing::info!("[MediaServerService] Server {} re-registered to cluster with updated config", updated.server_tag);
            }

            tracing::info!(
                "[MediaServerService] Updated server: tag={}",
                updated.server_tag
            );
            return Some(updated);
        }

        tracing::warn!("[MediaServerService] Server not found for update: tag={}", tag);
        None
    }

    /// 删除服务器
    ///
    /// # 返回
    /// 是否成功删除
    pub async fn delete(&self, tag: &str) -> bool {
        if let Some((_, ms)) = self.servers.remove(tag) {
            // 从集群注销
            self.cluster.unregister_server(tag);
            tracing::info!("[MediaServerService] Server {} unregistered from cluster", tag);
            
            if let Err(e) = self.repo.delete_server(ms.id).await {
                tracing::warn!(
                    "[MediaServerService] Failed to delete server from DB: {}",
                    e
                );
            }
            tracing::info!(
                "[MediaServerService] Deleted server: tag={}, id={}",
                tag, ms.id
            );
            return true;
        }

        tracing::warn!("[MediaServerService] Server not found for delete: tag={}", tag);
        false
    }

    /// 更新服务器启用状态
    pub fn update_enabled(&self, tag: &str, enabled: bool) -> Option<ManagedServer> {
        if let Some(mut entry) = self.servers.get_mut(tag) {
            entry.enabled = enabled;
            let updated = entry.value().clone();
            drop(entry);

            tracing::info!(
                "[MediaServerService] Server {} {}abled",
                tag,
                if enabled { "en" } else { "dis" }
            );
            return Some(updated);
        }
        None
    }

    /// 获取所有服务器配置（仅启用的）
    pub fn get_all_configs(&self) -> Vec<MediaServerConfig> {
        self.servers
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.value().to_config())
            .collect()
    }

    /// 获取所有启用的服务器
    pub fn get_enabled_servers(&self) -> Vec<ManagedServer> {
        self.servers
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.value().clone())
            .collect()
    }

    /// 服务器总数
    pub fn count(&self) -> usize {
        self.servers.len()
    }

    /// 在线服务器数（启用的）
    pub fn count_online(&self) -> usize {
        self.servers.iter().filter(|r| r.enabled).count()
    }

    /// 检查服务器是否存在
    pub fn exists(&self, tag: &str) -> bool {
        self.servers.contains_key(tag)
    }

    /// 获取服务器统计信息
    pub fn get_stats(&self) -> serde_json::Value {
        let total = self.servers.len();
        let enabled = self.servers.iter().filter(|r| r.enabled).count();
        let types: std::collections::HashMap<String, usize> = self
            .servers
            .iter()
            .fold(std::collections::HashMap::new(), |mut acc, r| {
                *acc.entry(r.server_type.to_string()).or_insert(0) += 1;
                acc
            });

        serde_json::json!({
            "total": total,
            "enabled": enabled,
            "disabled": total - enabled,
            "by_type": types,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_repo() -> Arc<DbRepository> {
        // 测试环境中创建 mock 或使用内存数据库
        unimplemented!("Use test database for integration tests")
    }

    #[tokio::test]
    async fn test_create_and_get_server() {
        // 集成测试示例
        // let repo = create_test_repo();
        // let service = MediaServerService::new(repo, vec![]);
        // let server = service.create(
        //     "test-server".to_string(),
        //     "http://localhost:8081".to_string(),
        //     "test-key".to_string(),
        //     ServerType::Zlmediakit,
        //     100,
        //     "test-tag".to_string(),
        //     ServerProtocolPorts::default(),
        // ).await;
        // assert_eq!(server.name, "test-server");
        // assert!(server.id > 0);
    }
}