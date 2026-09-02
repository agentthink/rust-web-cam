use std::sync::Arc;
use crate::domain::{Channel, DeviceStatus};
use crate::error::Result;

pub struct ChannelService {
    repo: Arc<crate::infrastructure::DbRepository>,
}

impl ChannelService {
    pub fn new(repo: Arc<crate::infrastructure::DbRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_channel(&self, device_tag: &str, channel_tag: &str) -> Result<Option<Channel>> {
        self.repo.get_channel(device_tag, channel_tag).await
    }

    pub async fn list_channels(&self) -> Result<Vec<Channel>> {
        self.repo.list_all_channels().await
    }

    pub fn list_channels_cached(&self) -> Vec<Channel> {
        self.repo.list_all_channels_cached()
    }

    pub fn list_channels_paginated(&self, limit: usize, offset: usize) -> Vec<Channel> {
        let all = self.repo.list_all_channels_cached();
        all.into_iter().skip(offset).take(limit).collect()
    }

    pub fn count_channels(&self) -> usize {
        self.repo.list_all_channels_cached().len()
    }

    pub fn get_channels_by_device(&self, device_tag: &str) -> Vec<Channel> {
        self.repo.get_channels_by_device_tag_cached(device_tag)
    }

    pub async fn create_channel(&self, channel: &Channel) -> Result<i64> {
        let id = self.repo.create_channel(channel).await?;
        Ok(id)
    }

    pub async fn update_channel(&self, channel: &Channel) -> Result<()> {
        self.repo.update_channel(channel).await
    }

    pub async fn get_or_create_channel(
        &self,
        device_tag: &str,
        channel_tag: &str,
        name: &str,
    ) -> Result<Channel> {
        if let Some(channel) = self.repo.get_channel(device_tag, channel_tag).await? {
            return Ok(channel);
        }

        let channel = Channel::new(
            device_tag.to_string(),
            channel_tag.to_string(),
            name.to_string(),
        );
        let id = self.repo.create_channel(&channel).await?;
        
        let mut created = channel;
        created.id = id;
        Ok(created)
    }
}
