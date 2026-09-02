use std::sync::Arc;
use crate::context::{InfraContext, MediaContext};
use crate::domain::device::RecordingConfig;
use crate::domain::recording::{
    CreateRecordingRequest, Recording, RecordingFormat, RecordingState,
};
use crate::error::{AppError, Result};

/// 录制管理服务
///
/// 负责视频录制的创建、开始、停止、暂停、恢复和文件管理。
///
/// # 依赖
/// - InfraContext (仅 db)
/// - MediaContext (仅 cluster)
pub struct RecordingService {
    repo: Arc<crate::infrastructure::DbRepository>,
    cluster: Arc<crate::infrastructure::cluster::ClusterManager>,
}

impl RecordingService {
    pub fn new(
        repo: Arc<crate::infrastructure::DbRepository>,
        cluster: Arc<crate::infrastructure::cluster::ClusterManager>,
    ) -> Self {
        Self {
            repo,
            cluster,
        }
    }

    /// 创建录制记录
    pub async fn create_recording(
        &self,
        req: CreateRecordingRequest,
        channel_tag: String,
        media_server: String,
    ) -> Recording {
        let format = req.format.unwrap_or(RecordingFormat::Mp4);
        let mut recording = Recording::new(req.device_tag.clone(), channel_tag, media_server, format);

        if let Some(labels) = req.labels {
            recording.labels = labels;
        }

        if let Err(e) = self.repo.create_recording(&recording).await {
            tracing::error!("[RecordingService] Failed to persist recording {}: {}", recording.id, e);
        }

        tracing::info!("[RecordingService] Created recording {} for device {}", recording.id, req.device_tag);
        recording
    }

    /// 获取录制
    pub async fn get_recording(&self, id: i64) -> Result<Option<Recording>> {
        self.repo.get_recording(id).await
    }

    /// 开始录制
    pub async fn start_recording(&self, id: i64) -> Result<Recording> {
        let mut recording = self.repo.get_recording(id).await?
            .ok_or_else(|| AppError::NotFound(format!("Recording {} not found", id)))?;

        let format_str = Self::format_to_str(recording.format.clone());

        let cache_key = format!("{}/{}", recording.device_tag.as_deref().unwrap_or(""), recording.channel_tag.as_deref().unwrap_or(""));
        let app = self.repo.streams_cache().get(&cache_key)
            .map(|s| s.app.clone())
            .unwrap_or_else(|| "live".to_string());

        let rec_info = if let Some(adapter) = self.cluster.get_server(&recording.media_server_name) {
            match adapter.start_recording(&app, &recording.stream_key(), format_str, recording.output_path.as_deref()).await {
                Ok(info) => {
                    tracing::info!(
                        "[RecordingService] Media server started recording: {} -> {}",
                        recording.stream_key(), info.output_path
                    );
                    Some(info)
                }
                Err(e) => {
                    tracing::error!(
                        "[RecordingService] Failed to start recording on media server {}: {}",
                        recording.media_server_name, e
                    );
                    recording.set_error(e.to_string());
                    if let Err(e) = self.repo.update_recording(&recording.clone()).await {
                        tracing::error!("[RecordingService] Failed to persist error state: {}", e);
                    }
                    return Err(AppError::Internal(format!("Media server error: {}", e)));
                }
            }
        } else {
            tracing::warn!(
                "[RecordingService] Media server {} not found",
                recording.media_server_name
            );
            None
        };

        recording.start();
        if let Some(info) = &rec_info {
            if !info.output_path.is_empty() {
                recording.set_output(info.output_path.clone(), 0);
            }
        }
        if let Err(e) = self.repo.update_recording(&recording.clone()).await {
            tracing::error!("[RecordingService] Failed to persist: {}", e);
        }

        tracing::info!("[RecordingService] Started recording {} (id={})", recording.stream_key(), id);
        Ok(recording)
    }

    /// 停止录制
    pub async fn stop_recording(&self, id: i64) -> Result<Recording> {
        let mut recording = self.repo.get_recording(id).await?
            .ok_or_else(|| AppError::NotFound(format!("Recording {} not found", id)))?;

        let format_str = Self::format_to_str(recording.format.clone());

        let cache_key = format!("{}/{}", recording.device_tag.as_deref().unwrap_or(""), recording.channel_tag.as_deref().unwrap_or(""));
        let app = self.repo.streams_cache().get(&cache_key)
            .map(|s| s.app.clone())
            .unwrap_or_else(|| "live".to_string());

        if let Some(adapter) = self.cluster.get_server(&recording.media_server_name) {
            if let Err(e) = adapter.stop_recording(&app, &recording.stream_key(), format_str).await {
                tracing::warn!("[RecordingService] Failed to stop recording on media server: {}", e);
            }
        }

        recording.stop();
        if let Err(e) = self.repo.update_recording(&recording.clone()).await {
            tracing::error!("[RecordingService] Failed to persist: {}", e);
        }

        tracing::info!("[RecordingService] Stopped recording {}", id);
        Ok(recording)
    }

    /// 暂停录制
    pub async fn pause_recording(&self, id: i64) -> Result<Recording> {
        let mut recording = self.repo.get_recording(id).await?
            .ok_or_else(|| AppError::NotFound(format!("Recording {} not found", id)))?;

        recording.pause();
        if let Err(e) = self.repo.update_recording(&recording.clone()).await {
            tracing::error!("[RecordingService] Failed to persist: {}", e);
        }

        Ok(recording)
    }

    /// 恢复录制
    pub async fn resume_recording(&self, id: i64) -> Result<Recording> {
        let mut recording = self.repo.get_recording(id).await?
            .ok_or_else(|| AppError::NotFound(format!("Recording {} not found", id)))?;

        recording.resume();
        if let Err(e) = self.repo.update_recording(&recording.clone()).await {
            tracing::error!("[RecordingService] Failed to persist: {}", e);
        }

        Ok(recording)
    }

    /// 删除录制
    pub async fn delete_recording(&self, id: i64) -> Result<()> {
        if self.repo.get_recording(id).await?.is_none() {
            return Err(AppError::NotFound(format!("Recording {} not found", id)));
        }

        if let Err(e) = self.repo.delete_recording(id).await {
            tracing::error!("[RecordingService] Failed to delete from DB: {}", e);
        }

        tracing::info!("[RecordingService] Deleted recording {}", id);
        Ok(())
    }

    /// 根据流标识停止录制
    pub async fn stop_recording_by_stream_key(&self, stream_key: &str) -> Result<()> {
        if let Some(recording) = self.get_active_recording_by_stream_key(stream_key).await? {
            self.stop_recording(recording.id).await?;
            tracing::info!("[RecordingService] Auto-stopped recording for stream={}", stream_key);
        }
        Ok(())
    }

    /// 为流自动开始录制
    pub async fn start_recording_for_stream(
        &self,
        device_tag: String,
        stream_key: &str,
        media_server: &str,
        config: &RecordingConfig,
    ) -> Result<Option<Recording>> {
        if !config.enabled {
            return Ok(None);
        }

        let channel_tag = crate::domain::stream::parse_stream_key(stream_key)
            .map(|(_, ch)| ch)
            .unwrap_or_else(|| "recording".to_string());

        let format = config.format.clone().unwrap_or(RecordingFormat::Mp4);
        let req = CreateRecordingRequest {
            device_tag,
            channel_tag: channel_tag.clone(),
            format: Some(format),
            duration_secs: config.max_duration_secs,
            max_file_size_mb: config.max_file_size_mb,
            output_path: None,
            labels: config.labels.clone(),
        };

        let recording = self.create_recording(req, channel_tag, media_server.to_string()).await;
        let recording = self.start_recording(recording.id).await?;

        tracing::info!("[RecordingService] Auto-started recording {} for stream={}", recording.id, stream_key);
        Ok(Some(recording))
    }

    /// 获取活跃录制
    pub async fn get_active_recording_by_stream_key(&self, stream_key: &str) -> Result<Option<Recording>> {
        self.repo.find_active_recording_by_stream_key(stream_key).await
    }

    /// 获取设备的所有录制
    pub async fn list_device_recordings(&self, device_tag: &str) -> Result<Vec<Recording>> {
        self.repo.list_recordings_by_device(device_tag).await
    }

    /// 获取设备的所有录制（通过device_tag）
    pub async fn list_device_recordings_by_device_tag(&self, device_tag: &str) -> Result<Vec<Recording>> {
        self.repo.list_recordings_by_device_tag(device_tag).await
    }

    /// 分页获取录制
    pub async fn list_recordings_paginated(&self, limit: usize, offset: usize) -> Result<Vec<Recording>> {
        self.repo.list_recordings_paginated(limit, offset).await
    }

    /// 获取录制总数
    pub async fn count_recordings(&self) -> Result<usize> {
        self.repo.count_recordings().await
    }

    /// 获取录制文件列表
    pub async fn list_recorded_files(&self, id: i64) -> Result<Vec<crate::adapter::media_server::RecordingFile>> {
        let recording = self.repo.get_recording(id).await?
            .ok_or_else(|| AppError::NotFound(format!("Recording {} not found", id)))?;

        let cache_key = format!("{}/{}", recording.device_tag.as_deref().unwrap_or(""), recording.channel_tag.as_deref().unwrap_or(""));
        let app = self.repo.streams_cache().get(&cache_key)
            .map(|s| s.app.clone())
            .unwrap_or_else(|| "live".to_string());

        if let Some(adapter) = self.cluster.get_server(&recording.media_server_name) {
            match adapter.list_recordings(&app, &recording.stream_key()).await {
                Ok(files) => return Ok(files),
                Err(e) => tracing::warn!("[RecordingService] Failed to list recordings: {}", e),
            }
        }

        Ok(vec![])
    }

    /// 获取录制统计
    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        let recordings = self.repo.list_recordings_paginated(10000, 0).await?;
        let total = recordings.len();
        let recording_count = recordings.iter()
            .filter(|r| r.state == RecordingState::Recording)
            .count();
        let completed = recordings.iter()
            .filter(|r| r.state == RecordingState::Completed)
            .count();
        let total_size: u64 = recordings.iter().map(|r| r.file_size).sum();

        Ok(serde_json::json!({
            "total": total,
            "recording": recording_count,
            "completed": completed,
            "total_size_bytes": total_size,
            "total_size_mb": total_size as f64 / (1024.0 * 1024.0),
        }))
    }

    pub async fn finalize_recording(
        &self,
        stream_key: &str,
        filename: String,
        file_size: u64,
        duration_secs: u64,
    ) -> Result<()> {
        if let Some(mut recording) = self.repo.find_active_recording_by_stream_key(stream_key).await? {
            recording.filename = Some(filename);
            recording.file_size = file_size;
            recording.duration_secs = duration_secs;
            recording.complete();
            if let Err(e) = self.repo.update_recording(&recording).await {
                tracing::error!("[RecordingService] Failed to finalize recording {}: {}", recording.id, e);
            } else {
                tracing::info!("[RecordingService] Finalized recording for stream={}", stream_key);
            }
        } else {
            tracing::warn!("[RecordingService] No active recording found for stream={}", stream_key);
        }
        Ok(())
    }

    fn format_to_str(format: RecordingFormat) -> &'static str {
        match format {
            RecordingFormat::Mp4 => "mp4",
            RecordingFormat::Hls => "hls",
            RecordingFormat::Flv => "flv",
            RecordingFormat::Ts => "ts",
        }
    }
}

impl Clone for RecordingService {
    fn clone(&self) -> Self {
        Self {
            repo: self.repo.clone(),
            cluster: self.cluster.clone(),
        }
    }
}