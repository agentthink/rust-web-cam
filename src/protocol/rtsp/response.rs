pub struct RtspResponse;

const SESSION_TIMEOUT_SECS: u32 = 60;

pub fn session_header(session: &str) -> String {
    format!("{};timeout={}", session, SESSION_TIMEOUT_SECS)
}

impl RtspResponse {
    pub fn ok(cseq: u32) -> String {
        format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\n\r\n", cseq)
    }

    pub fn options(cseq: u32) -> String {
        format!(
            "RTSP/1.0 200 OK\r\nCSeq: {}\r\nPublic: DESCRIBE, SETUP, PLAY, PAUSE, TEARDOWN, ANNOUNCE, GET_PARAMETER\r\n\r\n",
            cseq
        )
    }

    pub fn describe(cseq: u32, sdp: &str) -> String {
        format!(
            "RTSP/1.0 200 OK\r\nCSeq: {}\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{}",
            cseq, sdp.len(), sdp
        )
    }

    pub fn setup(cseq: u32, session: &str, transport: &str, track_id: Option<&str>) -> String {
        let mut lines = vec![
            format!("RTSP/1.0 200 OK"),
            format!("CSeq: {}", cseq),
            format!("Session: {}", session_header(session)),
        ];
        if let Some(tid) = track_id {
            lines.push(format!("Transport: {};trackID={}", transport, tid));
        } else {
            lines.push(format!("Transport: {}", transport));
        }
        lines.push(String::new());
        lines.join("\r\n") + "\r\n"
    }

    pub fn play(cseq: u32, session: &str, range: Option<&str>, rtp_info: Option<&str>) -> String {
        let range_line = range.unwrap_or("npt=0.000-");
        let rtp_info_line = rtp_info.map(|r| format!("RTP-Info: {}\r\n", r)).unwrap_or_default();
        format!(
            "RTSP/1.0 200 OK\r\nCSeq: {}\r\nSession: {}\r\nRange: {}\r\n{}r\n",
            cseq, session_header(session), range_line, rtp_info_line
        )
    }

    pub fn error(cseq: u32, code: u16, reason: &str) -> String {
        format!("RTSP/1.0 {} {}\r\nCSeq: {}\r\n\r\n", code, reason, cseq)
    }

    pub fn get_parameter(cseq: u32, session: &str) -> String {
        format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nSession: {}\r\nContent-Length: 0\r\n\r\n", cseq, session_header(session))
    }

    pub fn announce(cseq: u32) -> String {
        format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\n\r\n", cseq)
    }

    pub fn announce_with_transport(cseq: u32, transport: &str) -> String {
        format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nTransport: {}\r\n\r\n", cseq, transport)
    }

    pub fn unauthorized(cseq: u32, www_authenticate: &str) -> String {
        format!("RTSP/1.0 401 Unauthorized\r\nCSeq: {}\r\nWWW-Authenticate: {}\r\n\r\n", cseq, www_authenticate)
    }

    pub fn forbidden(cseq: u32) -> String {
        format!("RTSP/1.0 403 Forbidden\r\nCSeq: {}\r\n\r\n", cseq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_response() {
        let resp = RtspResponse::ok(1);
        assert!(resp.contains("200 OK"));
        assert!(resp.contains("CSeq: 1"));
    }

    #[test]
    fn test_options_response() {
        let resp = RtspResponse::options(2);
        assert!(resp.contains("Public:"));
    }

    #[test]
    fn test_describe_response() {
        let sdp = "v=0\r\ns=test";
        let resp = RtspResponse::describe(3, sdp);
        assert!(resp.contains("Content-Type: application/sdp"));
    }

    #[test]
    fn test_error_response() {
        let resp = RtspResponse::error(4, 404, "Not Found");
        assert!(resp.contains("404 Not Found"));
    }
}