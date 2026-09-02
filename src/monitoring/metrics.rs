use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct Metrics {
    inner: Arc<RwLock<MetricsInner>>,
}

struct MetricsInner {
    start_time: Instant,
    connections_total: u64,
    connections_active: u64,
    events_published: u64,
    events_received: u64,
    bytes_received: u64,
    bytes_sent: u64,
    devices_online: u64,
    devices_total: u64,
    sessions_active: u64,
    streams_active: u64,
    errors: HashMap<String, u64>,
    recovery_attempts: u64,
    recovery_success: u64,
    recovery_failures: u64,
    recovery_exhausted: u64,
    recovery_marked: u64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MetricsInner {
                start_time: Instant::now(),
                connections_total: 0,
                connections_active: 0,
                events_published: 0,
                events_received: 0,
                bytes_received: 0,
                bytes_sent: 0,
                devices_online: 0,
                devices_total: 0,
                sessions_active: 0,
                streams_active: 0,
                errors: HashMap::new(),
                recovery_attempts: 0,
                recovery_success: 0,
                recovery_failures: 0,
                recovery_exhausted: 0,
                recovery_marked: 0,
            })),
        }
    }

    pub fn connection_opened(&self) {
        let mut inner = self.inner.write();
        inner.connections_total += 1;
        inner.connections_active += 1;
    }

    pub fn connection_closed(&self) {
        let mut inner = self.inner.write();
        inner.connections_active = inner.connections_active.saturating_sub(1);
    }

    pub fn event_published(&self) {
        self.inner.write().events_published += 1;
    }

    pub fn event_received(&self) {
        self.inner.write().events_received += 1;
    }

    pub fn bytes_received(&self, n: usize) {
        self.inner.write().bytes_received += n as u64;
    }

    pub fn bytes_sent(&self, n: usize) {
        self.inner.write().bytes_sent += n as u64;
    }

    pub fn set_devices(&self, online: u64, total: u64) {
        let mut inner = self.inner.write();
        inner.devices_online = online;
        inner.devices_total = total;
    }

    pub fn set_sessions(&self, active: u64) {
        self.inner.write().sessions_active = active;
    }

    pub fn set_streams(&self, active: u64) {
        self.inner.write().streams_active = active;
    }

    pub fn record_error(&self, error_type: &str) {
        *self
            .inner
            .write()
            .errors
            .entry(error_type.to_string())
            .or_insert(0) += 1;
    }

    pub fn record_recovery_event(&self, event: &str) {
        let mut inner = self.inner.write();
        match event {
            "stream_recovery_attempts" => inner.recovery_attempts += 1,
            "stream_recovery_success" => inner.recovery_success += 1,
            "stream_recovery_failures" => inner.recovery_failures += 1,
            "stream_retries_exhausted" => inner.recovery_exhausted += 1,
            "stream_recovery_marked" => inner.recovery_marked += 1,
            _ => {}
        }
    }

    pub fn prometheus(&self) -> String {
        let m = self.inner.read();
        let uptime = m.start_time.elapsed().as_secs();
        let mut output = String::new();

        output.push_str(&format!("# HELP rustcam_uptime_seconds Server uptime\n# TYPE rustcam_uptime_seconds gauge\nrustcam_uptime_seconds {}\n", uptime));
        output.push_str(&format!("# HELP rustcam_connections_active Active connections\n# TYPE rustcam_connections_active gauge\nrustcam_connections_active {}\n", m.connections_active));
        output.push_str(&format!("# HELP rustcam_connections_total Total connections\n# TYPE rustcam_connections_total counter\nrustcam_connections_total {}\n", m.connections_total));
        output.push_str(&format!("# HELP rustcam_events_published Events published\n# TYPE rustcam_events_published counter\nrustcam_events_published {}\n", m.events_published));
        output.push_str(&format!("# HELP rustcam_events_received Events received\n# TYPE rustcam_events_received counter\nrustcam_events_received {}\n", m.events_received));
        output.push_str(&format!("# HELP rustcam_bytes_received Bytes received\n# TYPE rustcam_bytes_received counter\nrustcam_bytes_received {}\n", m.bytes_received));
        output.push_str(&format!("# HELP rustcam_bytes_sent Bytes sent\n# TYPE rustcam_bytes_sent counter\nrustcam_bytes_sent {}\n", m.bytes_sent));
        output.push_str(&format!("# HELP rustcam_devices_online Online devices\n# TYPE rustcam_devices_online gauge\nrustcam_devices_online {}\n", m.devices_online));
        output.push_str(&format!("# HELP rustcam_devices_total Total devices\n# TYPE rustcam_devices_total gauge\nrustcam_devices_total {}\n", m.devices_total));
        output.push_str(&format!("# HELP rustcam_sessions_active Active sessions\n# TYPE rustcam_sessions_active gauge\nrustcam_sessions_active {}\n", m.sessions_active));
        output.push_str(&format!("# HELP rustcam_streams_active Active streams\n# TYPE rustcam_streams_active gauge\nrustcam_streams_active {}\n", m.streams_active));
        output.push_str(&format!("# HELP rustcam_stream_recovery_attempts_total Recovery attempts\n# TYPE rustcam_stream_recovery_attempts_total counter\nrustcam_stream_recovery_attempts_total {}\n", m.recovery_attempts));
        output.push_str(&format!("# HELP rustcam_stream_recovery_success_total Recovery successes\n# TYPE rustcam_stream_recovery_success_total counter\nrustcam_stream_recovery_success_total {}\n", m.recovery_success));
        output.push_str(&format!("# HELP rustcam_stream_recovery_failures_total Recovery failures\n# TYPE rustcam_stream_recovery_failures_total counter\nrustcam_stream_recovery_failures_total {}\n", m.recovery_failures));
        output.push_str(&format!("# HELP rustcam_stream_retries_exhausted_total Streams with retries exhausted\n# TYPE rustcam_stream_retries_exhausted_total counter\nrustcam_stream_retries_exhausted_total {}\n", m.recovery_exhausted));

        for (error_type, count) in &m.errors {
            output.push_str(&format!(
                "rustcam_errors{{type=\"{}\"}} {}\n",
                error_type, count
            ));
        }

        output
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
