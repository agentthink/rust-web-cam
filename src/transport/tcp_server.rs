use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, BufReader};
use crate::protocol::adapter::AdapterRegistry;
use crate::protocol::traits::ProtocolDeps;
use crate::infrastructure::event_bus::EventBus;
use crate::monitoring::Metrics;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub max_packet_size: usize,
    pub read_buffer_size: usize,
    pub connection_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8080".to_string(),
            max_packet_size: 1024 * 1024,
            read_buffer_size: 65536,
            connection_timeout: 60,
        }
    }
}

pub struct TcpServer {
    listener: TcpListener,
    registry: Arc<AdapterRegistry>,
    event_bus: Arc<EventBus>,
    config: ServerConfig,
    metrics: Arc<Metrics>,
    protocol_deps: ProtocolDeps,
}

impl TcpServer {
    pub async fn new(
        config: ServerConfig,
        registry: Arc<AdapterRegistry>,
        event_bus: Arc<EventBus>,
        metrics: Arc<Metrics>,
        protocol_deps: ProtocolDeps,
    ) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(&config.bind_addr).await?;
        tracing::info!("[TcpServer] Bound to {}", config.bind_addr);
        Ok(Self {
            listener,
            registry,
            event_bus,
            config,
            metrics,
            protocol_deps,
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        tracing::info!("[TcpServer] Listening on {}", self.config.bind_addr);

        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    self.metrics.connection_opened();

                    let registry = self.registry.clone();
                    let event_bus = self.event_bus.clone();
                    let config = self.config.clone();
                    let metrics = self.metrics.clone();
                    let deps = self.protocol_deps.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream, addr, registry, event_bus, config, metrics.clone(), deps,
                        ).await {
                            tracing::error!("[TcpServer] Connection {} error: {}", addr, e);
                        }
                        metrics.connection_closed();
                    });
                }
                Err(e) => {
                    tracing::error!("[TcpServer] Accept error: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        mut stream: TcpStream,
        addr: SocketAddr,
        registry: Arc<AdapterRegistry>,
        event_bus: Arc<EventBus>,
        config: ServerConfig,
        metrics: Arc<Metrics>,
        deps: ProtocolDeps,
    ) -> anyhow::Result<()> {
        let mut peek_buf = vec![0u8; 1024];

        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.peek(&mut peek_buf),
        ).await {
            Ok(Ok(n)) if n > 0 => {
                metrics.bytes_received(n);

                let protocol = match registry.match_protocol(&peek_buf[..n]) {
                    Some(p) => p,
                    None => {
                        tracing::warn!("[TcpServer] Unknown protocol from {}: {:?}", addr, &peek_buf[..n.min(100)]);
                        return Ok(());
                    }
                };

                tracing::info!("[TcpServer] Detected {:?} from {}", protocol, addr);

                let mut adapter = match registry.create_adapter(&protocol, deps) {
                    Some(a) => a,
                    None => {
                        tracing::warn!("[TcpServer] No adapter for {:?}", protocol);
                        return Ok(());
                    }
                };
                
                adapter.on_connected(addr).await?;

                let (reader, write_half) = stream.into_split();
                adapter.set_tcp_write(write_half);

                let mut reader = BufReader::new(reader);
                let mut buffer = vec![0u8; config.read_buffer_size];

                loop {
                    tokio::select! {
                        result = reader.read(&mut buffer) => {
                            match result {
                                Ok(0) => {
                                    tracing::debug!("[TcpServer] Connection {} closed by peer", addr);
                                    break;
                                }
                                Ok(n) => {
                                    metrics.bytes_received(n);
                                    match adapter.parse(&buffer[..n]).await {
                                        Ok(events) => {
                                            for event in events {
                                                metrics.event_received();
                                                if let Err(e) = event_bus.publish(event).await {
                                                    tracing::error!("[TcpServer] Failed to publish event: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("[TcpServer] Parse error from {}: {}", addr, e);
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("[TcpServer] Read error from {}: {}", addr, e);
                                    break;
                                }
                            }
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(config.connection_timeout)) => {
                            tracing::warn!("[TcpServer] Connection {} timeout", addr);
                            break;
                        }
                    }
                }

                let _ = adapter.on_disconnected().await;
            }
            Ok(Ok(0)) => {
                tracing::debug!("[TcpServer] Empty connection from {}", addr);
            }
            Ok(Err(e)) => {
                tracing::error!("[TcpServer] Peek error from {}: {}", addr, e);
            }
            Err(_) => {
                tracing::warn!("[TcpServer] Connection timeout from {}", addr);
            }
            _ => {}
        }

        Ok(())
    }
}