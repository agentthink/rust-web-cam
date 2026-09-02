use std::sync::Arc;
use crate::domain::player_layout::{CreateLayoutRequest, PlayerLayout, UpdateLayoutRequest};
use crate::error::Result;

pub struct PlayerLayoutService {
    repo: Arc<crate::infrastructure::DbRepository>,
}

impl PlayerLayoutService {
    pub fn new(repo: Arc<crate::infrastructure::DbRepository>) -> Self { Self { repo } }

    pub async fn create(&self, req: CreateLayoutRequest) -> Result<PlayerLayout> {
        let layout_json = serde_json::to_value(&req.layout_json).unwrap_or(serde_json::Value::Array(vec![]));
        let id = self.repo.create_layout(&req.name, req.rows, req.cols, layout_json, req.is_default).await?;
        self.repo.get_layout(id).await.ok_or_else(|| crate::error::AppError::Internal("Failed to fetch created layout".to_string()))
    }

    pub async fn get(&self, id: i32) -> Result<Option<PlayerLayout>> {
        Ok(self.repo.get_layout(id as i64).await)
    }

    pub async fn list(&self) -> Result<Vec<PlayerLayout>> {
        Ok(self.repo.list_layouts().await)
    }

    pub async fn set_default(&self, id: i32) -> Result<()> {
        self.repo.set_default_layout(id as i64).await
    }

    pub async fn update(&self, id: i32, req: UpdateLayoutRequest) -> Result<()> {
        let layout_json = req.layout_json.as_ref().map(|j| serde_json::to_value(j).unwrap_or_default());
        self.repo.update_layout(id as i64, req.name, req.rows, req.cols, layout_json, req.is_default).await
    }

    pub async fn delete(&self, id: i32) -> Result<()> {
        self.repo.delete_layout(id as i64).await
    }
}