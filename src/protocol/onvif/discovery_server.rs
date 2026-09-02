use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;

const MULTICAST_ADDR: &str = "239.255.255.250";
const DISCOVERY_PORT: u16 = 3702;

pub struct OnvifDiscoveryServer {
    x_addr: String,
    scopes: Vec<String>,
    shutdown_rx: broadcast::Receiver<()>,
    device_id: String,
}

impl OnvifDiscoveryServer {
    pub fn new(local_ip: String, x_port: u16, scopes: Vec<String>) -> (Self, broadcast::Sender<()>) {
        let x_addr = format!("http://{}:{}/onvif/device_service", local_ip, x_port);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let device_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
        (Self { x_addr, scopes, shutdown_rx, device_id }, shutdown_tx)
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let bind_addr = format!("0.0.0.0:{}", DISCOVERY_PORT);
        let sock = UdpSocket::bind(&bind_addr).await?;
        sock.join_multicast_v4(MULTICAST_ADDR.parse().unwrap(), "0.0.0.0".parse().unwrap())?;

        tracing::info!("[ONVIF Discovery] Server listening on {}:{}", MULTICAST_ADDR, DISCOVERY_PORT);

        let hello = self.build_hello();
        let _ = sock.send_to(hello.as_bytes(), format!("{}:{}", MULTICAST_ADDR, DISCOVERY_PORT)).await;

        let mut buf = vec![0u8; 65536];
        loop {
            tokio::select! {
                res = sock.recv_from(&mut buf) => {
                    match res {
                        Ok((len, addr)) => {
                            let xml = String::from_utf8_lossy(&buf[..len]);
                            if xml.contains("Probe") {
                                let msg_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
                                let reply = self.build_probe_matches(&msg_id, &msg_id);
                                let _ = sock.send_to(reply.as_bytes(), addr).await;
                            }
                        }
                        Err(e) => tracing::warn!("[ONVIF Discovery] recv error: {}", e),
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    let bye = self.build_bye();
                    let _ = sock.send_to(bye.as_bytes(), format!("{}:{}", MULTICAST_ADDR, DISCOVERY_PORT)).await;
                    tracing::info!("[ONVIF Discovery] shutting down");
                    break;
                }
            }
        }
        Ok(())
    }

    fn build_probe_matches(&self, msg_id: &str, relates_to: &str) -> String {
        let scopes_str = self.scopes.join(" ");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://www.w3.org/2005/08/addressing" xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
<s:Header>
  <a:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches</a:Action>
  <a:MessageID>{}</a:MessageID>
  <a:RelatesTo>{}</a:RelatesTo>
  <a:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>
</s:Header>
<s:Body>
  <wsd:ProbeMatches>
    <wsd:ProbeMatch>
      <a:Address>{}</a:Address>
      <wsd:Types>dn:NetworkVideoTransmitter</wsd:Types>
      <wsd:Scopes>{}</wsd:Scopes>
      <wsd:XAddrs>{}</wsd:XAddrs>
      <wsd:MetadataVersion>1</wsd:MetadataVersion>
    </wsd:ProbeMatch>
  </wsd:ProbeMatches>
</s:Body>
</s:Envelope>"#,
            msg_id, relates_to, self.device_id, scopes_str, self.x_addr
        )
    }

    fn build_hello(&self) -> String {
        let msg_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
        let scopes_str = self.scopes.join(" ");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://www.w3.org/2005/08/addressing" xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
<s:Header>
  <a:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Hello</a:Action>
  <a:MessageID>{}</a:MessageID>
  <a:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>
</s:Header>
<s:Body>
  <wsd:Hello>
    <a:Address>{}</a:Address>
    <wsd:Types>dn:NetworkVideoTransmitter</wsd:Types>
    <wsd:Scopes>{}</wsd:Scopes>
    <wsd:XAddrs>{}</wsd:XAddrs>
    <wsd:MetadataVersion>1</wsd:MetadataVersion>
  </wsd:Hello>
</s:Body>
</s:Envelope>"#,
            msg_id, self.device_id, scopes_str, self.x_addr
        )
    }

    fn build_bye(&self) -> String {
        let msg_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://www.w3.org/2005/08/addressing" xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
<s:Header>
  <a:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Bye</a:Action>
  <a:MessageID>{}</a:MessageID>
  <a:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>
</s:Header>
<s:Body>
  <wsd:Bye><a:Address>{}</a:Address></wsd:Bye>
</s:Body>
</s:Envelope>"#,
            msg_id, self.device_id
        )
    }
}