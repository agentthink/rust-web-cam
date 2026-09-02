use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use tokio::net::UdpSocket;

use crate::protocol::adapter::AdapterRegistry;
use crate::protocol::adapter::SignalAdapter;

use crate::protocol::event::{SignalEvent, ProtocolType};
use crate::protocol::traits::ProtocolDeps;
use crate::infrastructure::event_bus::EventBus;
use crate::monitoring::Metrics;

#[derive(Clone)]
pub struct UdpServerConfig {
    pub bind_addr: String,
    pub max_packet_size: usize,
    pub adapter_timeout_secs: u64,
}

impl Default for UdpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8081".to_string(),
            max_packet_size: 65536,
            adapter_timeout_secs: 300,
        }
    }
}

struct UdpAdapterEntry {
    adapter: Arc<tokio::sync::Mutex<Box<dyn SignalAdapter + Send + Sync>>>,
    last_active: std::sync::atomic::AtomicI64,
}

pub struct UdpServer {
    socket: Arc<UdpSocket>,
    registry: Arc<AdapterRegistry>,
    event_bus: Arc<EventBus>,
    config: UdpServerConfig,
    metrics: Arc<Metrics>,
    deps: ProtocolDeps,
    adapters: Arc<DashMap<String, Arc<UdpAdapterEntry>>>,
}

impl UdpServer {
    pub async fn new(
        config: UdpServerConfig,
        registry: Arc<AdapterRegistry>,
        event_bus: Arc<EventBus>,
        metrics: Arc<Metrics>,
        deps: ProtocolDeps,
    ) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(&config.bind_addr).await?;
        tracing::info!("[UdpServer] Bound to {}", config.bind_addr);
        Ok(Self {
            socket: Arc::new(socket),
            registry,
            event_bus,
            config,
            metrics,
            deps,
            adapters: Arc::new(DashMap::new()),
        })
    }

    pub fn with_socket(
        config: UdpServerConfig,
        socket: Arc<UdpSocket>,
        registry: Arc<AdapterRegistry>,
        event_bus: Arc<EventBus>,
        metrics: Arc<Metrics>,
        deps: ProtocolDeps,
    ) -> Self {
        Self {
            socket,
            registry,
            event_bus,
            config,
            metrics,
            deps,
            adapters: Arc::new(DashMap::new()),
        }
    }

    pub fn socket(&self) -> Arc<UdpSocket> {
        self.socket.clone()
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        tracing::info!("[UdpServer] Listening on {}", self.config.bind_addr);
        self.start_cleanup_task();

        let mut buf = vec![0u8; self.config.max_packet_size];

        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    self.metrics.bytes_received(n);
                    let data = buf[..n].to_vec();
                    let addr_key = addr.to_string();

                    tracing::info!("[UdpServer] Received {} bytes from {}\n{}", n, addr_key, String::from_utf8_lossy(&data));

                    let entry = if let Some(existing) = self.adapters.get(&addr_key) {
                        tracing::debug!("[UdpServer] Found existing adapter for {}", addr_key);
                        existing.value().clone()
                    } else {
                        tracing::debug!("[UdpServer] No existing adapter for {}, matching protocol", addr_key);
                        let protocol = match self.registry.match_protocol(&data) {
                            Some(p) => {
                                tracing::debug!("[UdpServer] Protocol matched: {:?} from data: {:?}", p, &data[..data.len().min(50)]);
                                p
                            },
                            None => {
                                tracing::warn!("[UdpServer] Unknown protocol from {}: {:?}", addr, &data[..data.len().min(100)]);
                                continue;
                            }
                        };
                        self.get_or_create_adapter(&addr_key, &protocol).await
                    };
                    entry.last_active.store(chrono::Utc::now().timestamp(), std::sync::atomic::Ordering::Relaxed);

                    let adapter = entry.adapter.clone();
                    let mut guard = adapter.lock().await;

                    match guard.parse(&data).await {
                        Ok(events) => {
                            for event in events {
                                self.metrics.event_received();

                                if matches!(&event,
                                    SignalEvent::StopPlay { .. } |
                                    SignalEvent::DeviceOffline { .. }
                                ) {
                                    tracing::info!("[UdpServer] Device {} sent disconnect, removing adapter", addr_key);
                                    let _ = guard.on_disconnected().await;
                                    drop(guard);
                                    self.adapters.remove(&addr_key);
                                    let _ = self.event_bus.publish(event).await;
                                    break;
                                }

                                if let Err(e) = self.event_bus.publish(event).await {
                                    tracing::error!("[UdpServer] Failed to publish event: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("[UdpServer] Parse error from {}: {}", addr, e);
                            let _ = guard.on_disconnected().await;
                            drop(guard);
                            self.adapters.remove(&addr_key);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[UdpServer] Recv error: {}", e);
                }
            }
        }
    }

    async fn get_or_create_adapter(
        &self,
        addr_key: &str,
        protocol: &ProtocolType,
    ) -> Arc<UdpAdapterEntry> {
        if let Some(entry) = self.adapters.get(addr_key) {
            tracing::debug!("[UdpServer] get_or_create: found existing for {}", addr_key);
            return entry.value().clone();
        }

        tracing::info!("[UdpServer] Creating new adapter for {} with protocol {:?}", addr_key, protocol);

        let mut adapter = self.registry.create_adapter(protocol, self.deps.clone())
            .expect("No adapter for protocol");

        let addr: std::net::SocketAddr = addr_key.parse().unwrap();
        let _ = adapter.set_udp_peer(addr).await;
        let _ = adapter.on_connected(addr).await;

        let entry = Arc::new(UdpAdapterEntry {
            adapter: Arc::new(tokio::sync::Mutex::new(adapter)),
            last_active: std::sync::atomic::AtomicI64::new(chrono::Utc::now().timestamp()),
        });

        self.adapters.insert(addr_key.to_string(), entry.clone());
        entry
    }

    fn start_cleanup_task(&self) {
        let adapters = self.adapters.clone();
        let timeout_secs = self.config.adapter_timeout_secs as i64;
        let event_bus = self.event_bus.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp();
                let expired: Vec<String> = adapters.iter()
                    .filter(|entry| {
                        let last = entry.value().last_active.load(std::sync::atomic::Ordering::Relaxed);
                        now - last > timeout_secs
                    })
                    .map(|entry| entry.key().clone())
                    .collect();

                for key in expired {
                    if let Some((_, entry)) = adapters.remove(&key) {
                        tracing::info!("[UdpServer] Adapter {} expired, cleaning up", key);
                        let mut guard = entry.adapter.lock().await;
                        let _ = guard.on_disconnected().await;
                        let event = SignalEvent::DeviceOffline {
                            device_id: 0,
                            device_tag: Some(key.clone()),
                            reason: Some("UDP timeout".to_string()),
                        };
                        let _ = event_bus.publish(event).await;
                    }
                }
            }
        });
    }
}