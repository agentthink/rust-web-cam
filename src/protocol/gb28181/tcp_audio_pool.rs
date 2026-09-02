use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, mpsc};
use tokio::time::{timeout, Duration};
use std::sync::atomic::{AtomicU16, Ordering};

#[derive(Debug, Clone)]
pub struct TcpAudioServerConfig {
    pub port_start: u16,
    pub port_end: u16,
    pub idle_timeout_secs: u64,
    pub max_servers: usize,
}

impl Default for TcpAudioServerConfig {
    fn default() -> Self {
        Self {
            port_start: 15000,
            port_end: 15100,
            idle_timeout_secs: 30,
            max_servers: 10,
        }
    }
}

#[derive(Debug)]
pub struct TcpAudioConnection {
    pub device_tag: String,
    pub stream: Arc<RwLock<TcpStream>>,
    pub local_port: u16,
    pub connected_at: std::time::Instant,
}

impl Clone for TcpAudioConnection {
    fn clone(&self) -> Self {
        Self {
            device_tag: self.device_tag.clone(),
            stream: self.stream.clone(),
            local_port: self.local_port,
            connected_at: self.connected_at,
        }
    }
}

pub struct TcpAudioServerPool {
    config: TcpAudioServerConfig,
    next_port: AtomicU16,
    servers: Arc<RwLock<HashMap<u16, Arc<TcpListener>>>>,
    connections: Arc<RwLock<HashMap<u16, TcpAudioConnection>>>,
}

impl TcpAudioServerPool {
    pub fn new(config: TcpAudioServerConfig) -> Self {
        Self {
            next_port: AtomicU16::new(config.port_start),
            config,
            servers: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_next_available_port(&self) -> Option<u16> {
        let start = self.config.port_start;
        let end = self.config.port_end;
        let mut current = self.next_port.load(Ordering::SeqCst);
        
        for _ in 0..(end - start) {
            if current > end {
                current = start;
            }
            
            let servers = self.servers.read().await;
            if !servers.contains_key(&current) {
                drop(servers);
                self.next_port.store(current.wrapping_add(1), Ordering::SeqCst);
                return Some(current);
            }
            
            current = current.wrapping_add(1);
        }
        
        None
    }

    pub async fn start_server(&self, device_tag: &str) -> anyhow::Result<(u16, mpsc::Receiver<TcpAudioConnection>)> {
        let port = self.get_next_available_port().await
            .ok_or_else(|| anyhow::anyhow!("No available ports in range"))?;
        
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        let listener = Arc::new(listener);
        
        tracing::info!("[TCP Audio] Started server on port {} for device {}", port, device_tag);
        
        self.servers.write().await.insert(port, listener.clone());
        
        let (tx, rx) = mpsc::channel(1);
        let device_tag = device_tag.to_string();
        let connections = self.connections.clone();
        let servers = self.servers.clone();
        let idle_timeout = Duration::from_secs(self.config.idle_timeout_secs);
        let port_copy = port;
        
        tokio::spawn(async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!("[TCP Audio] Device {} connected from {} on port {}", device_tag, remote_addr, port);
                    
                    let conn = TcpAudioConnection {
                        device_tag: device_tag.clone(),
                        stream: Arc::new(RwLock::new(stream)),
                        local_port: port_copy,
                        connected_at: std::time::Instant::now(),
                    };
                    
                    connections.write().await.insert(port_copy, conn.clone());
                    
                    if tx.send(conn).await.is_err() {
                        tracing::error!("[TCP Audio] Failed to send connection for device {}", device_tag);
                    }
                    
                    // Wait for idle timeout, then cleanup
                    tokio::time::sleep(idle_timeout).await;
                    
                    tracing::info!("[TCP Audio] Connection timeout for device {} on port {}", device_tag, port);
                }
                Err(e) => {
                    tracing::error!("[TCP Audio] Failed to accept on port {}: {}", port, e);
                }
            }
            
            connections.write().await.remove(&port);
            servers.write().await.remove(&port);
            tracing::info!("[TCP Audio] Cleaned up server on port {} for device {}", port, device_tag);
        });
        
        Ok((port, rx))
    }

    pub async fn get_connection(&self, port: u16) -> Option<TcpAudioConnection> {
        self.connections.read().await.get(&port).cloned()
    }

    pub async fn remove_connection(&self, port: u16) {
        self.connections.write().await.remove(&port);
    }

    pub async fn stop_server(&self, port: u16) {
        self.servers.write().await.remove(&port);
        self.connections.write().await.remove(&port);
        tracing::info!("[TCP Audio] Stopped server on port {}", port);
    }

    pub async fn stop_all(&self) {
        let ports: Vec<u16> = self.servers.read().await.keys().cloned().collect();
        for port in ports {
            self.stop_server(port).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_audio_server_pool() {
        let config = TcpAudioServerConfig {
            port_start: 16000,
            port_end: 16010,
            idle_timeout_secs: 2,
            max_servers: 3,
        };
        
        let pool = TcpAudioServerPool::new(config);
        
        let (port, _rx) = pool.start_server("test-device-1").await.unwrap();
        assert!(port >= 16000 && port <= 16010);
        
        pool.stop_server(port).await;
        assert!(pool.servers.read().await.get(&port).is_none());
    }
}
