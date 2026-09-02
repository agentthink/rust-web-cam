use std::sync::Arc;
use tokio::signal;
use tokio::net::TcpListener;
use tracing::{info, error, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

use rustcam_media::context::{init_registry, ServiceRegistry};
use rustcam_media::config::AppConfig;
use rustcam_media::protocol::{
    AdapterRegistry, ProtocolMatcher, ProtocolType, SignalAdapter,
    gb28181::Gb28181Adapter,
    onvif::OnvifAdapter,
    rtsp::RtspServerAdapter,
    websocket::WebSocketAdapter,
};
use rustcam_media::protocol::traits::ProtocolDeps;
use rustcam_media::transport::{
    TcpServer, ServerConfig, UdpServer, UdpServerConfig,
};
use rustcam_media::api::{AppState, create_router, FullState};

/// 优雅关闭信号处理
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received SIGTERM signal");
        },
    }
}

/// 创建协议适配器注册表
fn create_adapter_registry(_deps: ProtocolDeps) -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();

    // GB28181
    registry.register_with_deps(
        |deps, register_fn, unregister_fn| {
            Box::new(Gb28181Adapter::new(deps, register_fn, unregister_fn)) as Box<dyn SignalAdapter + 'static>
        },
        ProtocolType::Gb28181,
        "GB28181",
    );
    registry.register_matcher(ProtocolMatcher::gb28181(), ProtocolType::Gb28181);

    // ONVIF
    registry.register_with_deps(
        |deps, _, _| Box::new(OnvifAdapter::new(deps)) as Box<dyn SignalAdapter + 'static>,
        ProtocolType::Onvif,
        "ONVIF",
    );
    registry.register_matcher(ProtocolMatcher::onvif(), ProtocolType::Onvif);

    // RTSP
    registry.register_with_deps(
        |deps, _, _| Box::new(RtspServerAdapter::new(deps)) as Box<dyn SignalAdapter + 'static>,
        ProtocolType::Rtsp,
        "RTSP-Server",
    );
    registry.register_matcher(ProtocolMatcher::rtsp(), ProtocolType::Rtsp);

    // WebSocket
    registry.register_with_deps(
        |deps, _, _| Box::new(WebSocketAdapter::new(deps)) as Box<dyn SignalAdapter + 'static>,
        ProtocolType::WebRtc,
        "WebSocket",
    );
    registry.register_matcher(ProtocolMatcher::websocket(), ProtocolType::WebRtc);

    registry
}

/// 初始化日志系统
fn init_tracing(level: &str) {
    let log_level = match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    LogTracer::init().expect("Failed to init LogTracer");
}

/// 编译时检查 FullState 是否满足 trait bounds
#[allow(dead_code)]
fn assert_fullstate_bounds() {
    fn assert_impl<T: Clone + Send + Sync + 'static>() {}
    assert_impl::<FullState>();
}

/// 主函数
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 0. Initialize JWT CryptoProvider (required for jsonwebtoken 11.x)
    jsonwebtoken::crypto::aws_lc::DEFAULT_PROVIDER.install_default()
        .expect("Failed to install JWT CryptoProvider");

    // 1. 加载配置
    let config = AppConfig::load()?;

    // 2. 初始化日志系统
    init_tracing(&config.log.level);
    info!("Starting RustCam-Media v{}", env!("CARGO_PKG_VERSION"));
    info!("Server: {}:{}", config.server.host, config.server.port);

    // 3. 创建 ServiceRegistry
    info!("Initializing services...");
    let registry = Arc::new(ServiceRegistry::new(config).await?);
    init_registry(registry.clone());
    registry.start_all().await;
    info!("All services started");

    // 4. 创建协议适配器
    let adapter_registry = Arc::new(create_adapter_registry(registry.protocol_deps.clone()));
    info!("Protocol adapters registered: {:?}", adapter_registry.registered_protocols());

    // 5. 构建 HTTP 路由
    let auth_state = registry.get_auth_state().await?;
    let app = create_router(AppState::from_registry(&registry), auth_state);

    // 6. 启动 HTTP 服务器
    let http_addr = format!(
        "{}:{}",
        registry.infra.config().server.host,
        registry.infra.config().server.port,
    );
    let listener = TcpListener::bind(&http_addr).await?;
    info!("HTTP API server listening on {}", http_addr);

    // 7. TCP 信令服务器
    let tcp_config = ServerConfig {
        bind_addr: format!(
            "{}:{}",
            registry.infra.config().signaling_server.bind_ip,
            registry.infra.config().signaling_server.tcp_signaling_port,
        ),
        max_packet_size: 1024 * 1024,
        read_buffer_size: 65536,
        connection_timeout: 60,
    };
    let tcp_server = TcpServer::new(
        tcp_config.clone(),
        adapter_registry.clone(),
        registry.infra.event_bus.clone(),
        registry.infra.metrics.clone(),
        registry.protocol_deps.clone(),
    )
        .await?;
    info!("TCP signaling server listening on {}", tcp_config.bind_addr);

    // 8. UDP 服务器
    let udp_config = UdpServerConfig {
        bind_addr: format!(
            "{}:{}",
            registry.infra.config().signaling_server.bind_ip,
            registry.infra.config().signaling_server.udp_signaling_port,
        ),
        max_packet_size: 65536,
        adapter_timeout_secs: 120,
    };
    let udp_socket = tokio::net::UdpSocket::bind(&udp_config.bind_addr).await?;
    let udp_socket = Arc::new(udp_socket);

    let udp_server = UdpServer::with_socket(
        udp_config.clone(),
        udp_socket.clone(),
        adapter_registry.clone(),
        registry.infra.event_bus.clone(),
        registry.infra.metrics.clone(),
        registry.protocol_deps.clone(),
    );
    info!("UDP server listening on {}", udp_config.bind_addr);

    // 9. GB28181 全局配置

    // 10. 后台 nonce 清理任务（每 5 分钟清理过期 nonce）
    let _nonce_cleanup = tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            rustcam_media::protocol::gb28181::auth::cleanup_expired_nonces();
            rustcam_media::protocol::rtsp::auth::cleanup_expired_nonces();
            tracing::debug!("[Cleanup] Nonce stores cleaned up");
        }
    });

    // 10. RTSP 会话超时清理任务（每 60 秒）
    let rtsp_cluster = registry.media.cluster.clone();
    let _rtsp_cleanup = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let removed = rustcam_media::protocol::rtsp::cleanup_expired(rtsp_cluster.clone()).await;
            if removed > 0 {
                tracing::debug!("[RTSP] Cleaned up {} expired sessions", removed);
            }
        }
    });

    // 11. GB28181 订阅超时清理任务（每 5 分钟）
    let _gb28181_sub_cleanup = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            let removed = rustcam_media::protocol::cleanup_expired_subscriptions_all().await;
            if removed > 0 {
                tracing::debug!("[GB28181] Cleaned up {} expired subscriptions", removed);
            }
        }
    });

    rustcam_media::protocol::gb28181::set_udp_sender(udp_socket);
    rustcam_media::protocol::gb28181::set_gb28181_platform_config(
        registry.infra.config().signaling_server.server_gb_id.clone(),
        registry.infra.config().signaling_server.server_gb_domain.clone(),
        registry.infra.config().signaling_server.bind_ip.clone(),
        registry.infra.config().signaling_server.tcp_signaling_port,
    );
    rustcam_media::protocol::gb28181::init_tcp_audio_server_pool(
        rustcam_media::protocol::gb28181::TcpAudioServerConfig {
            port_start: registry.infra.config().signaling_server.tcp_audio_port_start,
            port_end: registry.infra.config().signaling_server.tcp_audio_port_end,
            idle_timeout_secs: registry.infra.config().signaling_server.tcp_audio_idle_timeout_secs,
            max_servers: registry.infra.config().signaling_server.tcp_audio_max_servers,
        }
    );

    // 10. 并发运行所有服务
    let http_server = axum::serve(listener, app);

    tokio::select! {
        result = http_server => {
            if let Err(e) = result {
                error!("HTTP server error: {}", e);
            }
        }
        result = tcp_server.run() => {
            if let Err(e) = result {
                error!("TCP server error: {}", e);
            }
        }
        result = udp_server.run() => {
            if let Err(e) = result {
                error!("UDP server error: {}", e);
            }
        }
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
        }
    }

    // 11. 优雅关闭
    info!("Initiating graceful shutdown...");
    registry.graceful_shutdown().await;


    info!("RustCam-Media stopped");
    Ok(())
}