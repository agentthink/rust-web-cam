use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade, Path};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use crate::api::FullState;
use crate::application::ws_broadcaster::WsBroadcaster;
use crate::error::AppError;
use crate::protocol::adapter_manager;

/// WebSocket 连接处理器
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<FullState>>,
) -> Result<axum::response::Response, AppError> {
    let broadcaster = state.app.registry.ws_broadcaster.clone();

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, broadcaster)))
}

/// 处理 WebSocket 连接
///
/// 同时处理：
/// - 接收客户端消息（Ping/Pong/Close）
/// - 发送服务端广播消息
async fn handle_socket(
    socket: WebSocket,
    broadcaster: Arc<WsBroadcaster>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = broadcaster.subscribe();

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        tracing::debug!("[WebSocket] Received: {}", text);
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("[WebSocket] Client closed connection");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("[WebSocket] Error: {}", e);
                        break;
                    }
                    None => {
                        tracing::info!("[WebSocket] Client disconnected");
                        break;
                    }
                    _ => {}
                }
            }

            broadcast_msg = broadcast_rx.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        let text = serde_json::to_string(&msg).unwrap_or_default();
                        if sender.send(Message::Text(text.into())).await.is_err() {
                            tracing::warn!("[WebSocket] Failed to send message, closing");
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[WebSocket] Lagged {} messages", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("[WebSocket] Broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    tracing::info!("[WebSocket] Connection closed");
}

/// WebSocket audio talk handler (Browser -> Device)
pub async fn audio_talk_handler(
    ws: WebSocketUpgrade,
    Path(device_tag): Path<String>,
    State(state): State<Arc<FullState>>,
) -> Result<axum::response::Response, AppError> {
    tracing::info!("[AudioTalk] WebSocket connection request for device {}", device_tag);
    Ok(ws.on_upgrade(move |socket| handle_audio_talk(socket, device_tag, state)))
}

async fn handle_audio_talk(socket: WebSocket, device_tag: String, state: Arc<FullState>) {
    let (mut sender, mut receiver) = socket.split();

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        tracing::debug!("[AudioTalk] Received {} bytes from browser for device {}", data.len(), device_tag);

                        if let Some(device) = state.app.registry.device_service.get_device_by_device_tag(&device_tag).ok().flatten() {
                            if let Some(adapter_key) = device.device_tag.clone() {
                                if let Some(adapter) = adapter_manager::get_adapter(&adapter_key) {
                                    let mut guard = adapter.lock().await;

                                    let pcm_data: Vec<i16> = data.chunks_exact(2)
                                        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                                        .collect();

                                    if let Err(e) = guard.send_audio_to_device(&adapter_key, &pcm_data).await {
                                        tracing::error!("[AudioTalk] Failed to send audio: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        tracing::debug!("[AudioTalk] Received text: {}", text);
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("[AudioTalk] Client closed connection for device {}", device_tag);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("[AudioTalk] Error: {}", e);
                        break;
                    }
                    None => {
                        tracing::info!("[AudioTalk] Client disconnected for device {}", device_tag);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    tracing::info!("[AudioTalk] Connection closed for device {}", device_tag);
}
