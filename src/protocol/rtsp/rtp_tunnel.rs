use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use dashmap::DashMap;

/// RTP 隧道转发器
///
/// 将 RTSP TCP Interleaved 模式的 RTP 数据解包后，
/// 通过 UDP 转发到外部媒体服务器。
pub struct RtpTunnel {
    tunnels: Arc<DashMap<String, TunnelState>>,
}

#[derive(Clone)]
struct TunnelState {
    media_server_addr: SocketAddr,
    udp_socket: Arc<UdpSocket>,
    rtp_channel: u8,
    rtcp_channel: u8,
    rtp_packets: Arc<std::sync::atomic::AtomicU64>,
    rtcp_packets: Arc<std::sync::atomic::AtomicU64>,
    bytes_forwarded: Arc<std::sync::atomic::AtomicU64>,
}

const INTERLEAVED_HEADER_SIZE: usize = 4;

impl RtpTunnel {
    pub fn new() -> Self {
        Self { tunnels: Arc::new(DashMap::new()) }
    }

    pub async fn register(
        &self,
        stream_key: String,
        media_server_addr: SocketAddr,
        rtp_channel: u8,
    ) -> anyhow::Result<()> {
        let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);

        let state = TunnelState {
            media_server_addr, udp_socket,
            rtp_channel, rtcp_channel: rtp_channel + 1,
            rtp_packets: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            rtcp_packets: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            bytes_forwarded: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };

        self.tunnels.insert(stream_key.clone(), state);
        tracing::info!("[RtpTunnel] Registered: stream={}, dest={}, rtp_ch={}", stream_key, media_server_addr, rtp_channel);
        Ok(())
    }

    pub async fn handle_interleaved(
        &self, stream_key: &str, data: &[u8],
    ) -> anyhow::Result<Option<usize>> {
        if data.len() < INTERLEAVED_HEADER_SIZE { return Ok(None); }

        let magic = data[0];
        if magic != 0x24 { return Ok(None); }

        let channel = data[1];
        let payload_len = u16::from_be_bytes([data[2], data[3]]) as usize;
        let total_frame_len = INTERLEAVED_HEADER_SIZE + payload_len;

        if data.len() < total_frame_len { return Ok(None); }

        let tunnel = match self.tunnels.get(stream_key) {
            Some(t) => t,
            None => return Ok(Some(total_frame_len)),
        };

        let rtp_payload = &data[INTERLEAVED_HEADER_SIZE..total_frame_len];

        match tunnel.udp_socket.send_to(rtp_payload, tunnel.media_server_addr).await {
            Ok(sent) => {
                if channel == tunnel.rtp_channel {
                    tunnel.rtp_packets.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                } else if channel == tunnel.rtcp_channel {
                    tunnel.rtcp_packets.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                tunnel.bytes_forwarded.fetch_add(sent as u64, std::sync::atomic::Ordering::Relaxed);
            }
            Err(e) => {
                tracing::error!("[RtpTunnel] Forward failed: stream={}, err={}", stream_key, e);
                return Err(e.into());
            }
        }

        Ok(Some(total_frame_len))
    }

    pub fn unregister(&self, stream_key: &str) {
        if let Some((_, state)) = self.tunnels.remove(stream_key) {
            tracing::info!(
                "[RtpTunnel] Unregistered: stream={}, forwarded={} bytes",
                stream_key,
                state.bytes_forwarded.load(std::sync::atomic::Ordering::Relaxed),
            );
        }
    }

    pub fn tunnel_count(&self) -> usize { self.tunnels.len() }

    pub async fn shutdown(&self) {
        let keys: Vec<String> = self.tunnels.iter().map(|e| e.key().clone()).collect();
        for key in keys { self.unregister(&key); }
    }
}

impl Default for RtpTunnel {
    fn default() -> Self { Self::new() }
}