use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SipVia {
    pub protocol_name: String,
    pub protocol_version: String,
    pub transport: String,
    pub sent_by_host: String,
    pub sent_by_port: Option<u16>,
    pub ttl: Option<u32>,
    pub maddr: Option<String>,
    pub received: Option<String>,
    pub branch: Option<String>,
    pub extension: HashMap<String, String>,
}

impl SipVia {
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split_whitespace();
        let protocol = parts.next()?;
        let protocol_parts: Vec<&str> = protocol.split('/').collect();
        // SIP Via format: SIP/2.0/UDP or SIP/2.0/TCP
        let (protocol_name, protocol_version, transport) = match protocol_parts.len() {
            3 => (
                protocol_parts[0].to_string(),
                protocol_parts[1].to_string(),
                protocol_parts[2].to_string(),
            ),
            2 => (
                protocol_parts[0].to_string(),
                protocol_parts[1].to_string(),
                "UDP".to_string(),
            ),
            _ => return None,
        };

        let sent_by = parts.next()?;
        let (sent_by_host, port) = if let Some(pos) = sent_by.rfind(':') {
            let potential_port: &str = &sent_by[pos + 1..];
            if potential_port.parse::<u16>().is_ok() {
                (&sent_by[..pos], Some(potential_port.parse().ok()?))
            } else {
                (sent_by, None)
            }
        } else {
            (sent_by, None)
        };

        // sent_by might contain semicolon parameters like ";branch=..."
        let (final_sent_by, mut extension) = if let Some(semi_pos) = sent_by.find(';') {
            let host_part = &sent_by[..semi_pos];
            let params_part = &sent_by[semi_pos + 1..];
            let mut ext = HashMap::new();
            for param in params_part.split(';') {
                if let Some((k, v)) = param.split_once('=') {
                    ext.insert(k.trim().to_lowercase(), v.trim().to_string());
                }
            }
            (host_part, ext)
        } else {
            (sent_by, HashMap::new())
        };

        let mut via = Self {
            protocol_name,
            protocol_version,
            transport,
            sent_by_host: final_sent_by.to_string(),
            sent_by_port: port,
            ttl: None,
            maddr: None,
            received: None,
            branch: extension.remove("branch"),
            extension,
        };

        Some(via)
    }

    pub fn to_string(&self) -> String {
        let mut s = format!(
            "{}/{}/{} {}",
            self.protocol_name, self.protocol_version, self.transport, self.sent_by_host
        );
        if let Some(port) = self.sent_by_port {
            s.push(':');
            s.push_str(&port.to_string());
        }
        if let Some(ttl) = self.ttl {
            s.push_str(";ttl=");
            s.push_str(&ttl.to_string());
        }
        if let Some(ref maddr) = self.maddr {
            s.push_str(";maddr=");
            s.push_str(maddr);
        }
        if let Some(ref received) = self.received {
            s.push_str(";received=");
            s.push_str(received);
        }
        if let Some(ref branch) = self.branch {
            s.push_str(";branch=");
            s.push_str(branch);
        }
        for (k, v) in &self.extension {
            s.push(';');
            s.push_str(k);
            if !v.is_empty() {
                s.push('=');
                s.push_str(v);
            }
        }
        s
    }

    pub fn get_branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn get_received(&self) -> Option<&str> {
        self.received.as_deref()
    }

    pub fn get_rport(&self) -> Option<u16> {
        self.extension.get("rport").and_then(|v| {
            if v.is_empty() {
                Some(0)
            } else {
                v.parse().ok()
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct SipUri {
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub params: HashMap<String, String>,
}

impl SipUri {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches("sip:");
        let (user_part, host_part) = s.split_once('@')?;

        let host = if let Some(pos) = host_part.find(':') {
            if let Ok(port) = host_part[pos + 1..].parse::<u16>() {
                (host_part[..pos].to_string(), Some(port))
            } else {
                (host_part.to_string(), None)
            }
        } else {
            (host_part.to_string(), None)
        };

        let mut params = HashMap::new();
        if let Some(pos) = host.0.find(';') {
            for param in host.0[pos + 1..].split(';') {
                if let Some((k, v)) = param.split_once('=') {
                    params.insert(k.to_string(), v.to_string());
                }
            }
        }

        let user = if user_part.contains(';') {
            Some(user_part.split(';').next().unwrap_or("").to_string())
        } else {
            Some(user_part.to_string())
        };

        Some(Self {
            user,
            host: host.0.split(';').next().unwrap_or(&host.0).to_string(),
            port: host.1,
            params,
        })
    }

    pub fn to_string(&self) -> String {
        let mut s = String::from("sip:");
        if let Some(ref user) = self.user {
            s.push_str(user);
            s.push('@');
        }
        s.push_str(&self.host);
        if let Some(port) = self.port {
            s.push(':');
            s.push_str(&port.to_string());
        }
        for (k, v) in &self.params {
            s.push(';');
            s.push_str(k);
            s.push('=');
            s.push_str(v);
        }
        s
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SipMethod {
    Invite,
    Ack,
    Bye,
    Cancel,
    Options,
    Register,
    Subscribe,
    Notify,
    Message,
    Info,
    Prack,
    Update,
    Extension(String),
}

impl SipMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "INVITE" => Some(SipMethod::Invite),
            "ACK" => Some(SipMethod::Ack),
            "BYE" => Some(SipMethod::Bye),
            "CANCEL" => Some(SipMethod::Cancel),
            "OPTIONS" => Some(SipMethod::Options),
            "REGISTER" => Some(SipMethod::Register),
            "SUBSCRIBE" => Some(SipMethod::Subscribe),
            "NOTIFY" => Some(SipMethod::Notify),
            "MESSAGE" => Some(SipMethod::Message),
            "INFO" => Some(SipMethod::Info),
            "PRACK" => Some(SipMethod::Prack),
            "UPDATE" => Some(SipMethod::Update),
            _ => Some(SipMethod::Extension(s.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            SipMethod::Invite => "INVITE",
            SipMethod::Ack => "ACK",
            SipMethod::Bye => "BYE",
            SipMethod::Cancel => "CANCEL",
            SipMethod::Options => "OPTIONS",
            SipMethod::Register => "REGISTER",
            SipMethod::Subscribe => "SUBSCRIBE",
            SipMethod::Notify => "NOTIFY",
            SipMethod::Message => "MESSAGE",
            SipMethod::Info => "INFO",
            SipMethod::Prack => "PRACK",
            SipMethod::Update => "UPDATE",
            SipMethod::Extension(s) => s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SipRequestLine {
    pub method: SipMethod,
    pub uri: SipUri,
}

#[derive(Debug, Clone)]
pub struct SipStatusLine {
    pub version: String,
    pub status_code: u16,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SipNameAddr {
    pub uri: SipUri,
    pub display_name: Option<String>,
    pub params: HashMap<String, String>,
}

impl SipNameAddr {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let mut display_name = None;
        let mut remaining = s;

        if let Some(angle_pos) = s.find('<') {
            let before = s[..angle_pos].trim();
            if !before.is_empty() {
                display_name = Some(before.to_string());
            }
            if let Some(end) = s.find('>') {
                remaining = &s[angle_pos + 1..end];
            }
        } else {
            remaining = s;
        }

        let uri = SipUri::parse(remaining)?;
        let mut params = HashMap::new();

        if let Some(angle_end) = s.find('>') {
            let after_angle = &s[angle_end + 1..];
            for param in after_angle.split(';') {
                let param = param.trim();
                if let Some((k, v)) = param.split_once('=') {
                    params.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }

        Some(Self {
            uri,
            display_name,
            params,
        })
    }

    pub fn new(uri: SipUri) -> Self {
        Self {
            uri,
            display_name: None,
            params: HashMap::new(),
        }
    }

    pub fn with_display_name(mut self, name: &str) -> Self {
        self.display_name = Some(name.to_string());
        self
    }

    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();
        if let Some(ref dn) = self.display_name {
            s.push_str(dn);
            s.push(' ');
        }
        s.push('<');
        s.push_str(&self.uri.to_string());
        s.push('>');
        for (k, v) in &self.params {
            s.push(';');
            s.push_str(k);
            s.push('=');
            s.push_str(v);
        }
        s
    }

    pub fn get_tag(&self) -> Option<&str> {
        self.params.get("tag").map(|s| s.as_str())
    }

    pub fn get_param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct SipHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct SipHeaders {
    pub via_raw: Option<String>,
    pub via: Option<SipVia>,
    pub from: Option<SipNameAddr>,
    pub to: Option<SipNameAddr>,
    pub call_id: Option<String>,
    pub cseq_num: Option<u32>,
    pub cseq_method: Option<String>,
    pub content_length: Option<usize>,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub contact: Option<String>,
    pub expires: Option<u32>,
    pub event: Option<String>,
    pub subject: Option<String>,
    pub www_authenticate: Option<String>,
    pub authorization: Option<String>,
    pub server: Option<String>,
    pub allow: Option<String>,
    pub accept: Option<String>,
    pub others: HashMap<String, String>,
}

impl SipHeaders {
    pub fn new() -> Self {
        Self {
            via_raw: None,
            via: None,
            from: None,
            to: None,
            call_id: None,
            cseq_num: None,
            cseq_method: None,
            content_length: None,
            content_type: None,
            user_agent: None,
            contact: None,
            expires: None,
            event: None,
            subject: None,
            www_authenticate: None,
            authorization: None,
            server: None,
            allow: None,
            accept: None,
            others: HashMap::new(),
        }
    }

    pub fn parse(raw_headers: &str) -> Self {
        let mut headers = Self::new();

        for line in raw_headers.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let name = k.trim().to_lowercase();
                let value = v.trim().to_string();

                match name.as_str() {
                    "via" => {
                        headers.via_raw = Some(value.clone());
                        headers.via = SipVia::parse(&value);
                    }
                    "from" => headers.from = SipNameAddr::parse(&value),
                    "to" => headers.to = SipNameAddr::parse(&value),
                    "call-id" | "callid" => headers.call_id = Some(value),
                    "cseq" => {
                        let parts: Vec<&str> = value.split_whitespace().collect();
                        if parts.len() >= 2 {
                            headers.cseq_num = parts[0].parse().ok();
                            headers.cseq_method = Some(parts[1].to_string());
                        }
                    }
                    "content-length" | "contentlength" => {
                        headers.content_length = value.parse().ok();
                    }
                    "content-type" | "contenttype" => headers.content_type = Some(value),
                    "user-agent" | "useragent" => headers.user_agent = Some(value),
                    "contact" => headers.contact = Some(value),
                    "expires" => headers.expires = value.parse().ok(),
                    "event" => headers.event = Some(value),
                    "subject" => headers.subject = Some(value),
                    "www-authenticate" | "wwwauthenticate" => {
                        headers.www_authenticate = Some(value)
                    }
                    "authorization" => headers.authorization = Some(value),
                    "server" => headers.server = Some(value),
                    "allow" => headers.allow = Some(value),
                    "accept" => headers.accept = Some(value),
                    _ => {
                        headers.others.insert(name, value);
                    }
                }
            }
        }

        headers
    }

    pub fn get(&self, name: &str) -> Option<String> {
        match name.to_lowercase().as_str() {
            "via" => self.via_raw.clone(),
            "from" => self.from.as_ref().map(|na| na.to_string()),
            "to" => self.to.as_ref().map(|na| na.to_string()),
            "call-id" | "callid" => self.call_id.clone(),
            "cseq" => {
                if let (Some(n), Some(ref m)) = (self.cseq_num, &self.cseq_method) {
                    Some(format!("{} {}", n, m))
                } else {
                    None
                }
            }
            "content-length" | "contentlength" => self.content_length.map(|v| v.to_string()),
            "content-type" | "contenttype" => self.content_type.clone(),
            "user-agent" | "useragent" => self.user_agent.clone(),
            "contact" => self.contact.clone(),
            "expires" => self.expires.map(|v| v.to_string()),
            "event" => self.event.clone(),
            "subject" => self.subject.clone(),
            "www-authenticate" | "wwwauthenticate" => self.www_authenticate.clone(),
            "authorization" => self.authorization.clone(),
            "server" => self.server.clone(),
            "allow" => self.allow.clone(),
            "accept" => self.accept.clone(),
            _ => self.others.get(name).cloned(),
        }
    }

    pub fn has_header(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn to_vec(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if let Some(ref v) = self.via_raw {
            result.push(("via".to_string(), v.clone()));
        }
        if let Some(ref v) = self.from {
            result.push(("from".to_string(), v.to_string()));
        }
        if let Some(ref v) = self.to {
            result.push(("to".to_string(), v.to_string()));
        }
        if let Some(ref v) = self.call_id {
            result.push(("call-id".to_string(), v.clone()));
        }
        if let (Some(n), Some(ref m)) = (self.cseq_num, &self.cseq_method) {
            result.push(("cseq".to_string(), format!("{} {}", n, m)));
        }
        if let Some(n) = self.content_length {
            result.push(("content-length".to_string(), n.to_string()));
        }
        if let Some(ref v) = self.content_type {
            result.push(("content-type".to_string(), v.clone()));
        }
        if let Some(ref v) = self.user_agent {
            result.push(("user-agent".to_string(), v.clone()));
        }
        if let Some(ref v) = self.contact {
            result.push(("contact".to_string(), v.clone()));
        }
        if let Some(n) = self.expires {
            result.push(("expires".to_string(), n.to_string()));
        }
        if let Some(ref v) = self.event {
            result.push(("event".to_string(), v.clone()));
        }
        if let Some(ref v) = self.subject {
            result.push(("subject".to_string(), v.clone()));
        }
        if let Some(ref v) = self.www_authenticate {
            result.push(("www-authenticate".to_string(), v.clone()));
        }
        if let Some(ref v) = self.authorization {
            result.push(("authorization".to_string(), v.clone()));
        }
        if let Some(ref v) = self.server {
            result.push(("server".to_string(), v.clone()));
        }
        if let Some(ref v) = self.allow {
            result.push(("allow".to_string(), v.clone()));
        }
        if let Some(ref v) = self.accept {
            result.push(("accept".to_string(), v.clone()));
        }
        for (k, v) in &self.others {
            result.push((k.clone(), v.clone()));
        }
        result
    }
}

impl Default for SipHeaders {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SipBody {
    pub raw: Option<String>,
    pub content_length: usize,
}

impl SipBody {
    pub fn new() -> Self {
        Self {
            raw: None,
            content_length: 0,
        }
    }

    pub fn with_content(mut self, raw: String) -> Self {
        self.raw = Some(raw.clone());
        self.content_length = raw.len();
        self
    }

    pub fn raw(&self) -> Option<&String> {
        self.raw.as_ref()
    }

    pub fn raw_or_default(&self) -> String {
        self.raw.clone().unwrap_or_default()
    }
}

impl Default for SipBody {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SipMessage {
    pub request_line: Option<SipRequestLine>,
    pub status_line: Option<SipStatusLine>,
    pub headers: SipHeaders,
    pub body: SipBody,
}

impl SipMessage {
    pub fn request(method: SipMethod, uri: SipUri) -> Self {
        Self {
            request_line: Some(SipRequestLine { method, uri }),
            status_line: None,
            headers: SipHeaders::new(),
            body: SipBody::new(),
        }
    }

    pub fn response(status_code: u16, reason: &str) -> Self {
        Self {
            request_line: None,
            status_line: Some(SipStatusLine {
                version: "SIP/2.0".to_string(),
                status_code,
                reason: reason.to_string(),
            }),
            headers: SipHeaders::new(),
            body: SipBody::new(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        let name_lower = name.to_lowercase();
        match name_lower.as_str() {
            "via" => {
                self.headers.via_raw = Some(value.to_string());
                self.headers.via = SipVia::parse(value);
            }
            "from" => self.headers.from = SipNameAddr::parse(value),
            "to" => self.headers.to = SipNameAddr::parse(value),
            "call-id" | "callid" => self.headers.call_id = Some(value.to_string()),
            "content-length" | "contentlength" => {
                self.headers.content_length = value.parse().ok();
            }
            "content-type" | "contenttype" => self.headers.content_type = Some(value.to_string()),
            "user-agent" | "useragent" => self.headers.user_agent = Some(value.to_string()),
            "contact" => self.headers.contact = Some(value.to_string()),
            "expires" => self.headers.expires = value.parse().ok(),
            "event" => self.headers.event = Some(value.to_string()),
            "subject" => self.headers.subject = Some(value.to_string()),
            "www-authenticate" | "wwwauthenticate" => {
                self.headers.www_authenticate = Some(value.to_string())
            }
            "authorization" => self.headers.authorization = Some(value.to_string()),
            "server" => self.headers.server = Some(value.to_string()),
            "allow" => self.headers.allow = Some(value.to_string()),
            "accept" => self.headers.accept = Some(value.to_string()),
            "cseq" => {
                if let Some(space_idx) = value.find(' ') {
                    if let Ok(num) = value[..space_idx].parse::<u32>() {
                        self.headers.cseq_num = Some(num);
                        self.headers.cseq_method = Some(value[space_idx + 1..].trim().to_string());
                    }
                }
            }
            _ => {
                self.headers.others.insert(name_lower, value.to_string());
            }
        }
        self
    }

    pub fn get_header(&self, name: &str) -> Option<String> {
        self.headers.get(name)
    }

    pub fn has_header(&self, name: &str) -> bool {
        self.headers.has_header(name)
    }

    pub fn set_body(mut self, body: &str) -> Self {
        self.body = SipBody::new().with_content(body.to_string());
        self.headers.content_length = Some(body.len());
        self
    }

    pub fn set_content_type(mut self, ct: &str) -> Self {
        self.headers.content_type = Some(ct.to_string());
        self
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();

        if let Some(ref rl) = self.request_line {
            s.push_str(rl.method.as_str());
            s.push(' ');
            s.push_str(&rl.uri.to_string());
            s.push_str(" SIP/2.0\r\n");
        } else if let Some(ref sl) = self.status_line {
            s.push_str("SIP/2.0 ");
            s.push_str(&sl.status_code.to_string());
            s.push(' ');
            s.push_str(&sl.reason);
            s.push_str("\r\n");
        }

        if let Some(ref via) = self.headers.via {
            s.push_str("Via: ");
            s.push_str(&via.to_string());
            s.push_str("\r\n");
        }
        if let Some(ref call_id) = self.headers.call_id {
            s.push_str("Call-ID: ");
            s.push_str(call_id);
            s.push_str("\r\n");
        }
        if let (Some(n), Some(ref m)) = (self.headers.cseq_num, &self.headers.cseq_method) {
            s.push_str("CSeq: ");
            s.push_str(&n.to_string());
            s.push(' ');
            s.push_str(m);
            s.push_str("\r\n");
        }
        if let Some(ref from) = self.headers.from {
            s.push_str("From: ");
            s.push_str(&from.to_string());
            s.push_str("\r\n");
        }
        if let Some(ref to) = self.headers.to {
            s.push_str("To: ");
            s.push_str(&to.to_string());
            s.push_str("\r\n");
        }
        if let Some(ref contact) = self.headers.contact {
            s.push_str("Contact: ");
            s.push_str(contact);
            s.push_str("\r\n");
        }
        if let Some(ref subject) = self.headers.subject {
            s.push_str("Subject: ");
            s.push_str(subject);
            s.push_str("\r\n");
        }
        if let Some(ref ua) = self.headers.user_agent {
            s.push_str("User-Agent: ");
            s.push_str(ua);
            s.push_str("\r\n");
        }
        if let Some(ref server) = self.headers.server {
            s.push_str("Server: ");
            s.push_str(server);
            s.push_str("\r\n");
        }
        if let Some(ref allow) = self.headers.allow {
            s.push_str("Allow: ");
            s.push_str(allow);
            s.push_str("\r\n");
        }
        if let Some(ref accept) = self.headers.accept {
            s.push_str("Accept: ");
            s.push_str(accept);
            s.push_str("\r\n");
        }
        if let Some(ref event) = self.headers.event {
            s.push_str("Event: ");
            s.push_str(event);
            s.push_str("\r\n");
        }
        if let Some(exp) = self.headers.expires {
            s.push_str("Expires: ");
            s.push_str(&exp.to_string());
            s.push_str("\r\n");
        }
        if let Some(ref wwauth) = self.headers.www_authenticate {
            s.push_str("WWW-Authenticate: ");
            s.push_str(wwauth);
            s.push_str("\r\n");
        }
        if let Some(ref auth) = self.headers.authorization {
            s.push_str("Authorization: ");
            s.push_str(auth);
            s.push_str("\r\n");
        }
        if let Some(ref ct) = self.headers.content_type {
            s.push_str("Content-Type: ");
            s.push_str(ct);
            s.push_str("\r\n");
        }
        if let Some(cl) = self.headers.content_length {
            s.push_str("Content-Length: ");
            s.push_str(&cl.to_string());
            s.push_str("\r\n");
        }

        for (name, value) in &self.headers.others {
            s.push_str(name);
            s.push_str(": ");
            s.push_str(value);
            s.push_str("\r\n");
        }

        s.push_str("\r\n");

        if let Some(ref body) = self.body.raw {
            s.push_str(body);
        }

        s
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let mut lines = raw.lines();

        let first_line = lines.next()?;
        let (request_line, status_line) = if first_line.starts_with("SIP/2.0") {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() >= 3 {
                let status_line = SipStatusLine {
                    version: parts[0].to_string(),
                    status_code: parts[1].parse().ok()?,
                    reason: parts[2..].join(" "),
                };
                (None, Some(status_line))
            } else {
                return None;
            }
        } else {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() >= 3 {
                let method = SipMethod::from_str(parts[0])?;
                let uri = SipUri::parse(parts[1])?;
                let request_line = SipRequestLine { method, uri };
                (Some(request_line), None)
            } else {
                return None;
            }
        };

        let raw_headers = lines.collect::<Vec<_>>().join("\r\n");
        let headers = SipHeaders::parse(&raw_headers);

        let body_raw = if let Some(pos) = raw_headers.find("\r\n\r\n") {
            Some(raw_headers[pos + 4..].to_string())
        } else {
            None
        };
        let body = if let Some(ref b) = body_raw {
            SipBody::new().with_content(b.clone())
        } else {
            SipBody::new()
        };

        Some(Self {
            request_line,
            status_line,
            headers,
            body,
        })
    }

    pub fn get_method(&self) -> Option<SipMethod> {
        self.request_line.as_ref().map(|rl| rl.method.clone())
    }

    pub fn get_status_code(&self) -> Option<u16> {
        self.status_line.as_ref().map(|sl| sl.status_code)
    }

    pub fn get_call_id(&self) -> Option<String> {
        self.headers.call_id.clone()
    }

    pub fn get_from(&self) -> Option<SipNameAddr> {
        self.headers.from.clone()
    }

    pub fn get_to(&self) -> Option<SipNameAddr> {
        self.headers.to.clone()
    }

    pub fn get_cseq(&self) -> Option<(String, u32)> {
        if let (Some(n), Some(ref m)) = (self.headers.cseq_num, &self.headers.cseq_method) {
            Some((m.clone(), n))
        } else {
            None
        }
    }

    pub fn get_via(&self) -> Option<String> {
        self.headers.via_raw.clone()
    }

    pub fn get_subject(&self) -> Option<String> {
        self.headers.subject.clone()
    }

    pub fn get_expires(&self) -> Option<u32> {
        self.headers.expires
    }

    pub fn get_event(&self) -> Option<String> {
        self.headers.event.clone()
    }

    pub fn get_via_received(&self) -> Option<String> {
        self.headers
            .via
            .as_ref()
            .and_then(|v| v.get_received().map(|s| s.to_string()))
    }

    pub fn get_via_rport(&self) -> Option<u16> {
        self.headers.via.as_ref().and_then(|v| v.get_rport())
    }

    pub fn get_via_branch(&self) -> Option<String> {
        self.headers
            .via
            .as_ref()
            .and_then(|v| v.get_branch().map(|s| s.to_string()))
    }
}

impl SipMessage {
    pub fn from_buffer(buffer: &[u8]) -> Option<(SipMessage, usize)> {
        let msg_str = String::from_utf8_lossy(buffer);
        let header_end = msg_str.find("\r\n\r\n")?;
        let header_section = &msg_str[..header_end];

        let content_length = header_section
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);

        let body_start = header_end + 4;
        let total_len = body_start + content_length;
        if buffer.len() < body_start {
            return None;
        }
        let actual_content_length =
            std::cmp::min(content_length, buffer.len().saturating_sub(body_start));

        let raw_msg = if actual_content_length > 0 || buffer.len() > body_start {
            let actual_end = std::cmp::min(total_len, buffer.len());
            &msg_str[..actual_end]
        } else {
            header_section
        };

        let msg = SipMessage::parse(raw_msg)?;
        Some((msg, body_start + actual_content_length))
    }

    pub fn to_old_format(&self) -> (String, Vec<(String, String)>, String) {
        let first_line = if let Some(ref rl) = self.request_line {
            format!("{} {} SIP/2.0", rl.method.as_str(), rl.uri.to_string())
        } else if let Some(ref sl) = self.status_line {
            format!("SIP/2.0 {} {}", sl.status_code, sl.reason)
        } else {
            String::new()
        };

        let headers = self.headers.to_vec();
        let body = self.body.raw_or_default();

        (first_line, headers, body)
    }

    pub fn get_first_line(&self) -> Option<String> {
        Some(if let Some(ref rl) = self.request_line {
            format!("{} {} SIP/2.0", rl.method.as_str(), rl.uri.to_string())
        } else if let Some(ref sl) = self.status_line {
            format!("SIP/2.0 {} {}", sl.status_code, sl.reason)
        } else {
            return None;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sip_uri_parse() {
        let uri = SipUri::parse("sip:64010000002000000001@192.168.1.100:5060").unwrap();
        assert_eq!(uri.user, Some("64010000002000000001".to_string()));
        assert_eq!(uri.host, "192.168.1.100".to_string());
        assert_eq!(uri.port, Some(5060));
    }

    #[test]
    fn test_sip_name_addr() {
        let na =
            SipNameAddr::parse("<sip:64010000002000000001@192.168.1.100:5060>;tag=abc123").unwrap();
        assert_eq!(na.uri.user, Some("64010000002000000001".to_string()));
        assert_eq!(na.params.get("tag"), Some(&"abc123".to_string()));
    }

    #[test]
    fn test_sip_message_parse() {
        let raw = "INVITE sip:64010000002000000001@192.168.1.100:5060 SIP/2.0\r\n\
            Via: SIP/2.0/UDP 192.168.1.1:5060;branch=z9hG4bKabc\r\n\
            From: <sip:64010000001000000001@192.168.1.1:5060>;tag=xyz\r\n\
            To: <sip:64010000002000000001@192.168.1.100:5060>\r\n\
            Call-ID: call123\r\n\
            CSeq: 1 INVITE\r\n\
            Content-Length: 0\r\n\
            \r\n";

        let msg = SipMessage::parse(raw).unwrap();
        assert_eq!(msg.get_method(), Some(SipMethod::Invite));
        assert_eq!(
            msg.get_header("via"),
            Some("SIP/2.0/UDP 192.168.1.1:5060;branch=z9hG4bKabc".to_string())
        );
        assert_eq!(msg.get_call_id(), Some("call123".to_string()));
    }

    #[test]
    fn test_sip_message_build() {
        let msg = SipMessage::request(
            SipMethod::Invite,
            SipUri::parse("64010000002000000001@192.168.1.100:5060").unwrap(),
        )
        .header("via", "SIP/2.0/UDP 192.168.1.1:5060;branch=z9hG4bKabc")
        .header(
            "from",
            "<sip:64010000001000000001@192.168.1.1:5060>;tag=xyz",
        )
        .header("to", "<sip:64010000002000000001@192.168.1.100:5060>")
        .header("call-id", "call123")
        .header("cseq", "1 INVITE")
        .header("user-agent", "RustCam-Media/2.0")
        .header("subject", "10000000001320000001:64010000001000000001:0")
        .set_content_type("application/sdp")
        .set_body("v=0\r\n");

        let s = msg.to_string();
        assert!(s.contains("INVITE sip:"));
        assert!(s.contains("SIP/2.0"));
        assert!(s.contains("Content-Type: application/sdp"));
        assert!(s.contains("Content-Length: 5"));
    }
}
