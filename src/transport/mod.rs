pub mod tcp_server;
pub mod udp_server;

pub use tcp_server::{TcpServer, ServerConfig};
pub use udp_server::{UdpServer, UdpServerConfig};

pub fn create_rtsp_server_config() -> ServerConfig {
    ServerConfig {
        bind_addr: "0.0.0.0:8554".to_string(),
        max_packet_size: 1024 * 1024,
        read_buffer_size: 65536,
        connection_timeout: 60,
    }
}