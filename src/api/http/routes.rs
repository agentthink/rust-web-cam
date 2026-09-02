use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::Path,
    middleware::Next,
    extract::Request,
};
use crate::api::state::FullState;
use crate::api::http::handlers::{
    alarm_handler,
    dashboard,
    device_handler,
    stream_handler,
    server_handler,
    session_handler,
    recording_handler,
    ptz_handler,
    layout_handler,
    region_handler,
    group_handler,
    onvif_handler,
    gb28181_ref_handler,
    channel_handler,
};

async fn trace_layer(request: Request, next: Next) -> axum::response::Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    tracing::debug!("[HTTP] => {} {}", method, uri);

    let response = next.run(request).await;

    let elapsed = start.elapsed();
    let status = response.status();

    tracing::debug!("[HTTP] <= {} {} ({:.2?})", status.as_u16(), uri, elapsed);

    response
}

/// 创建所有 API 路由
pub fn create_routes(state: FullState) -> Router {
    let state = Arc::new(state);

    let protected = protected_routes(state.clone());
    let public = public_routes(state);

    protected.merge(public)
}

/// 受保护的路由（需要 JWT 认证）
fn protected_routes(state: Arc<FullState>) -> Router {
    Router::new()
        // Dashboard
        .route("/api/v1/dashboard", get(dashboard::dashboard_handler))
        .route("/api/v1/admin/reload-cache", post(dashboard::reload_cache_handler))

        // 设备管理
        .route("/api/v1/devices", get(device_handler::list_devices_handler))
        .route("/api/v1/devices/online", get(device_handler::list_online_devices_handler))
        .route("/api/v1/devices", post(device_handler::create_device_handler))
        .route("/api/v1/devices/{id}", get(device_handler::get_device_handler))
        .route("/api/v1/devices/{id}", put(device_handler::update_device_handler))
        .route("/api/v1/devices/{id}", delete(device_handler::delete_device_handler))
        .route("/api/v1/devices/{id}/play", post(device_handler::play_device_handler))
        .route("/api/v1/devices/{id}/playback", post(device_handler::playback_device_handler))
        .route("/api/v1/devices/{id}/config", get(device_handler::get_device_config_handler))
        .route("/api/v1/devices/{id}/stop", post(device_handler::stop_device_handler))
        .route("/api/v1/devices/{id}/start", post(device_handler::start_device_handler))
        .route("/api/v1/devices/{id}/channels", get(channel_handler::get_device_channels_handler))

        // 通道管理
        .route("/api/v1/channels", get(channel_handler::list_channels_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}", get(channel_handler::get_channel_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/play-links", get(channel_handler::get_channel_play_links_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/status", get(channel_handler::get_channel_status_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/start", post(channel_handler::start_channel_stream_handler))
        // PTZ 控制（通道级）
        .route("/api/v1/channels/{device_tag}/{channel_tag}/ptz", post(ptz_handler::channel_ptz_control_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/ptz/presets", get(ptz_handler::get_channel_ptz_presets_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/ptz/presets", post(ptz_handler::create_channel_ptz_preset_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/ptz/presets/{token}", delete(ptz_handler::delete_channel_ptz_preset_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/ptz/presets/{token}", put(ptz_handler::rename_channel_ptz_preset_handler))
        .route("/api/v1/channels/{device_tag}/{channel_tag}/ptz/status", get(ptz_handler::get_channel_ptz_status_handler))

        // 报警管理
        .route("/api/v1/alarms", get(alarm_handler::list_alarms_handler))
        .route("/api/v1/alarms/{id}/processed", put(alarm_handler::mark_alarm_processed_handler))

        // ONVIF 发现与探测
        .route("/api/v1/onvif/discover", post(onvif_handler::onvif_discover_handler))
        .route("/api/v1/onvif/probe", post(onvif_handler::onvif_probe_handler))
        .route("/api/v1/onvif/capabilities", post(onvif_handler::onvif_capabilities_handler))
        .route("/api/v1/onvif/stream-uris", post(onvif_handler::onvif_stream_uris_handler))
        .route("/api/v1/onvif/devices", post(onvif_handler::onvif_create_devices_handler))
        .route("/api/v1/onvif/check-online", post(onvif_handler::onvif_check_online_handler))

        // 流管理
        .route("/api/v1/streams", get(stream_handler::list_streams_handler))
        .route("/api/v1/streams", post(stream_handler::start_stream_handler))
        .route("/api/v1/streams/{id}", get(stream_handler::get_stream_handler))
        .route("/api/v1/streams/{id}", delete(stream_handler::stop_stream_handler))
        .route("/api/v1/streams/{id}/play", get(stream_handler::get_stream_play_url))
        .route("/api/v1/streams/{id}/play-links", get(stream_handler::get_stream_play_links_handler))
        .route("/api/v1/streams/{id}/online", get(stream_handler::is_stream_online_handler))
        .route("/api/v1/streams/{id}/restart", post(stream_handler::restart_stream_handler))
        .route("/api/v1/streams/by-device/{device_id}", get(stream_handler::list_streams_by_device_handler))

        // 服务器管理
        .route("/api/v1/servers", get(server_handler::list_servers_handler))
        .route("/api/v1/servers", post(server_handler::create_server_handler))
        .route("/api/v1/servers/{tag}", get(server_handler::get_server_handler))
        .route("/api/v1/servers/{tag}", put(server_handler::update_server_handler))
        .route("/api/v1/servers/{tag}", delete(server_handler::delete_server_handler))
        .route("/api/v1/servers/{tag}/status", get(server_handler::server_status_handler))
        .route("/api/v1/servers/{tag}/refresh", post(server_handler::refresh_server_handler))
        .route("/api/v1/servers/{tag}/enable", post(server_handler::enable_server_handler))
        .route("/api/v1/servers/{tag}/disable", post(server_handler::disable_server_handler))
        .route("/api/v1/servers/{tag}/sessions", get(server_handler::server_sessions_handler))

        // 会话管理
        .route("/api/v1/sessions", get(session_handler::list_sessions_handler))

        // 录制管理
        .route("/api/v1/recordings", get(recording_handler::list_recordings_handler))
        .route("/api/v1/recordings", post(recording_handler::create_recording_handler))
        .route("/api/v1/recordings/files", get(recording_handler::all_recording_files_handler))
        .route("/api/v1/recordings/stats", get(recording_handler::recording_stats_handler))
        .route("/api/v1/recordings/{id}", get(recording_handler::get_recording_handler))
        .route("/api/v1/recordings/{id}/start", post(recording_handler::start_recording_handler))
        .route("/api/v1/recordings/{id}/stop", post(recording_handler::stop_recording_handler))
        .route("/api/v1/recordings/{id}", delete(recording_handler::delete_recording_handler))
        .route("/api/v1/recordings/{id}/pause", post(recording_handler::pause_recording_handler))
        .route("/api/v1/recordings/{id}/resume", post(recording_handler::resume_recording_handler))
        .route("/api/v1/recordings/{id}/files", get(recording_handler::recording_files_handler))

        // 播放器布局
        .route("/api/v1/layouts", get(layout_handler::list_layouts_handler))
        .route("/api/v1/layouts", post(layout_handler::create_layout_handler))
        .route("/api/v1/layouts/{id}", get(layout_handler::get_layout_handler))
        .route("/api/v1/layouts/{id}", put(layout_handler::update_layout_handler))
        .route("/api/v1/layouts/{id}", delete(layout_handler::delete_layout_handler))
        .route("/api/v1/layouts/{id}/default", put(layout_handler::set_default_layout_handler))

        // 区域管理
        .route("/api/v1/regions", get(region_handler::list_regions_handler))
        .route("/api/v1/regions/tree", get(region_handler::list_region_tree_handler))

        // 设备分组
        .route("/api/v1/groups/tree", get(group_handler::list_group_tree_handler))
        .route("/api/v1/groups", get(group_handler::list_groups_handler))
        .route("/api/v1/groups", post(group_handler::create_group_handler))
        .route("/api/v1/groups/{id}", put(group_handler::update_group_handler))
        .route("/api/v1/groups/{id}", delete(group_handler::delete_group_handler))
        .route("/api/v1/devices/{id}/group", put(group_handler::assign_device_group_handler))

        // GB28181 参考数据
        .route("/api/v1/gb28181/ref-data", get(gb28181_ref_handler::get_gb28181_ref_data_handler))

        // 认证用户管理
        .route("/api/v1/auth/me", get(crate::auth::handlers::me))
        .route("/api/v1/users", get(crate::auth::handlers::list_users))
        .route("/api/v1/users", post(crate::auth::handlers::create_user))
        .route("/api/v1/users/{id}", get(crate::auth::handlers::get_user))
        .route("/api/v1/users/{id}", put(crate::auth::handlers::update_user))
        .route("/api/v1/users/{id}", delete(crate::auth::handlers::delete_user))
        .route("/api/v1/users/{id}/roles", put(crate::auth::handlers::assign_user_roles))
        .route("/api/v1/roles", get(crate::auth::handlers::list_roles))
        .route("/api/v1/roles", post(crate::auth::handlers::create_role))
        .route("/api/v1/roles/{id}/permissions", put(crate::auth::handlers::set_role_permissions))
        .route("/api/v1/permissions", get(crate::auth::handlers::list_permissions))
        .route("/api/v1/permissions", post(crate::auth::handlers::create_permission))

        // HTTP trace layer (logs incoming requests at TRACE level)
        .layer(axum::middleware::from_fn(trace_layer))

        // JWT 认证中间件
        .route_layer(axum::middleware::from_fn_with_state(
            state.auth.clone(),
            crate::auth::middleware::jwt_auth_layer,
        ))

        .with_state(state)
}

/// 公开路由（无需认证）
fn public_routes(state: Arc<FullState>) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/metrics", get(metrics_handler))
        // ZLMediaKit hook 事件路由：/hook/{event_name}
        .route("/hook/{event_name}", get(hook_handler).post(hook_handler))
        // 兼容：所有事件发到同一个 URL
        .route("/hook", get(hook_handler).post(hook_handler))
        .route("/api/v1/auth/login", post(crate::auth::handlers::login))
        .route("/api/v1/auth/refresh", post(crate::auth::handlers::refresh))
        .route("/api/v1/public/streams", get(device_handler::list_public_streams_handler))
        .route("/api/v1/stats", get(stats_handler))
        .route("/ws", get(crate::api::websocket::ws_handler))
        .route("/ws/audio-talk/{device_id}", get(crate::api::websocket::audio_talk_handler))
        // HTTP trace layer (logs incoming requests at TRACE level)
        .layer(axum::middleware::from_fn(trace_layer))
        .with_state(state)
}

// ═══════════════════════════════════════════════════════════════
// 公开路由 Handlers
// ═══════════════════════════════════════════════════════════════

async fn root_handler() -> &'static str {
    "RustCam-Media API v2.0"
}

async fn health_handler() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}

use axum::{extract::State, Json, response::IntoResponse};
use axum::http::header;


async fn metrics_handler(
    State(state): State<Arc<FullState>>,
) -> impl IntoResponse {
    let output = state.app.registry.infra.metrics.prometheus();
    let mut response = output.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
}

async fn hook_handler(
    State(state): State<Arc<FullState>>,
    Path(event_name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    tracing::debug!("[Hook] Received event={}, body={}", event_name, serde_json::to_string_pretty(&body).unwrap_or_default());

    // 构建完整事件 JSON（注入 event 字段）
    let mut map = serde_json::Map::new();
    if let Some(obj) = body.as_object() {
        for (k, v) in obj {
            map.insert(k.clone(), v.clone());
        }
    }
    map.insert("event".to_string(), serde_json::Value::String(event_name.clone()));
    let event_json = serde_json::Value::Object(map);

    // 解析事件
    match serde_json::from_value::<crate::application::zlmediakit_hook::ZlMediaKitHookEvent>(event_json) {
        Ok(event) => {
            let response = state.app.registry.zlmediakit_hook_handler.handle(event).await;
            Json(serde_json::to_value(response).unwrap_or(serde_json::json!({ "code": 0, "msg": "ok" })))
        }
        Err(e) => {
            tracing::warn!("[Hook] Failed to parse event={}: {}", event_name, e);
            Json(serde_json::json!({ "code": 0, "msg": "ok" }))
        }
    }
}

async fn stats_handler(
    State(state): State<Arc<FullState>>,
) -> Json<serde_json::Value> {
    let registry = &state.app.registry;
    let device_stats = registry.device_service.get_stats();
    let stream_stats = registry.stream_service.get_stats().await;

    Json(serde_json::json!({
        "devices": device_stats,
        "streams": stream_stats,
    }))
}