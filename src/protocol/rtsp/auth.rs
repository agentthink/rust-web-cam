use base64::Engine;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

pub static NONCE_STORE: Lazy<DashMap<String, i64>> = Lazy::new(|| DashMap::new());
static NONCE_BLACKLIST: Lazy<DashMap<String, i64>> = Lazy::new(|| DashMap::new());
static NONCE_NC: Lazy<RwLock<HashMap<String, u32>>> = Lazy::new(|| RwLock::new(HashMap::new()));
const NONCE_TTL_SECS: i64 = 600;

pub fn cleanup_expired_nonces() {
    let now = chrono::Utc::now().timestamp();
    NONCE_STORE.retain(|_, expiry| *expiry > now);
    NONCE_BLACKLIST.retain(|_, expiry| *expiry > now);
    let mut nc = NONCE_NC.write().unwrap();
    nc.retain(|nonce, _| NONCE_STORE.contains_key(nonce));
}

#[derive(Debug, Clone)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct DigestAuth {
    pub username: String,
    pub realm: String,
    pub nonce: String,
    pub uri: String,
    pub response: String,
    pub algorithm: Option<String>,
    pub qop: Option<String>,
    pub nc: Option<String>,
    pub cnonce: Option<String>,
    pub opaque: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AuthHeader {
    Basic(BasicAuth),
    Digest(DigestAuth),
}

#[derive(Debug, Clone)]
pub struct RtspAuthContext {
    pub realm: String,
    pub default_username: Option<String>,
    pub default_password: Option<String>,
    pub enabled: bool,
}

impl RtspAuthContext {
    pub fn new(realm: &str) -> Self {
        Self {
            realm: realm.to_string(),
            default_username: None,
            default_password: None,
            enabled: true,
        }
    }

    pub fn with_defaults(mut self, username: Option<String>, password: Option<String>) -> Self {
        self.default_username = username;
        self.default_password = password;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub fn parse_authorization(header: &str) -> Option<AuthHeader> {
    let header = header.trim();
    if header.starts_with("Basic ") {
        let encoded = header.trim_start_matches("Basic ").trim();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?;
        let decoded_str = String::from_utf8_lossy(&decoded);
        let (username, password) = decoded_str.split_once(':')?;
        Some(AuthHeader::Basic(BasicAuth {
            username: username.to_string(),
            password: password.to_string(),
        }))
    } else if header.starts_with("Digest ") {
        let rest = header.trim_start_matches("Digest ").trim();
        let params = parse_digest_params(rest)?;
        Some(AuthHeader::Digest(DigestAuth {
            username: params.get("username")?.clone(),
            realm: params.get("realm").cloned().unwrap_or_default(),
            nonce: params.get("nonce")?.clone(),
            uri: params.get("uri")?.clone(),
            response: params.get("response")?.clone(),
            algorithm: params.get("algorithm").cloned(),
            qop: params.get("qop").cloned(),
            nc: params.get("nc").cloned(),
            cnonce: params.get("cnonce").cloned(),
            opaque: params.get("opaque").cloned(),
        }))
    } else {
        None
    }
}

fn parse_digest_params(input: &str) -> Option<HashMap<String, String>> {
    let mut params = HashMap::new();
    let mut remaining = input.trim();
    while !remaining.is_empty() {
        let equals = remaining.find('=')?;
        let raw_key = remaining[..equals].trim();
        let key = raw_key.trim_matches('"').trim_matches(',');
        remaining = &remaining[equals + 1..];
        let val = if remaining.starts_with('"') {
            let end_quote = remaining[1..].find('"')? + 2;
            let val = remaining[1..end_quote - 1].to_string();
            remaining = &remaining[end_quote..].trim_start_matches(',').trim_start();
            val
        } else {
            let end_space = remaining
                .find(|c: char| c.is_ascii_whitespace() || c == ',')
                .unwrap_or(remaining.len());
            let val = remaining[..end_space].trim_matches(',').to_string();
            remaining = &remaining[end_space..].trim_start_matches(',').trim_start();
            val
        };
        params.insert(key.to_string(), val);
    }
    Some(params)
}

pub fn verify_basic(auth: &AuthHeader, user: &str, pass: &str) -> bool {
    match auth {
        AuthHeader::Basic(b) => b.username == user && b.password == pass,
        _ => false,
    }
}

fn md5_hex(data: &str) -> String {
    format!("{:x}", md5::compute(data.as_bytes()))
}

pub fn verify_digest(auth: &AuthHeader, user: &str, pass: &str, method: &str, uri: &str) -> bool {
    let digest = match auth {
        AuthHeader::Digest(d) => d,
        _ => return false,
    };
    if digest.username != user {
        return false;
    }

    let has_qop = digest.qop.is_some();
    if has_qop {
        if digest.cnonce.is_none() || digest.nc.is_none() {
            return false;
        }
        if let Some(ref nc) = digest.nc {
            if nc.len() != 8 || !nc.chars().all(|c| c.is_ascii_hexdigit()) {
                return false;
            }
        }
    }

    if let Some(ref nc_val) = digest.nc {
        if !check_and_update_nc(&digest.nonce, nc_val) {
            return false;
        }
    }

    let computed = compute_digest_response(
        user,
        pass,
        &digest.realm,
        method,
        uri,
        &digest.nonce,
        digest.qop.as_deref(),
        digest.nc.as_deref(),
        digest.cnonce.as_deref(),
    );
    let valid = digest.response == computed;
    if valid {
        consume_nonce(&digest.nonce);
    }
    valid
}

pub fn compute_digest_response(
    user: &str,
    pass: &str,
    realm: &str,
    method: &str,
    uri: &str,
    nonce: &str,
    qop: Option<&str>,
    nc: Option<&str>,
    cnonce: Option<&str>,
) -> String {
    let ha1 = md5_hex(&format!("{}:{}:{}", user, realm, pass));
    let ha2 = md5_hex(&format!("{}:{}", method, uri));

    if qop.is_some() {
        let nc_part = nc.unwrap_or("00000001");
        let cnonce_part = cnonce.unwrap_or("");
        md5_hex(&format!(
            "{}:{}:{}:{}:{}:{}",
            ha1,
            nonce,
            nc_part,
            cnonce_part,
            qop.unwrap_or("auth"),
            ha2
        ))
    } else {
        md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2))
    }
}

pub fn generate_nonce() -> (String, i64) {
    let nonce = uuid::Uuid::new_v4().to_string();
    let expires = chrono::Utc::now().timestamp() + NONCE_TTL_SECS;
    NONCE_STORE.insert(nonce.clone(), expires);
    (nonce, expires)
}

pub fn verify_nonce(nonce: &str, expires: i64) -> bool {
    if NONCE_BLACKLIST.contains_key(nonce) {
        return false;
    }
    if chrono::Utc::now().timestamp() > expires {
        NONCE_STORE.remove(nonce);
        let mut nc = NONCE_NC.write().unwrap();
        nc.remove(nonce);
        return false;
    }
    true
}

pub fn check_and_update_nc(nonce: &str, nc_value: &str) -> bool {
    if let Ok(nc) = u32::from_str_radix(nc_value, 16) {
        let mut nc_map = NONCE_NC.write().unwrap();
        match nc_map.get(nonce) {
            Some(&last) if nc <= last => return false,
            _ => {
                nc_map.insert(nonce.to_string(), nc);
                return true;
            }
        }
    }
    false
}

pub fn consume_nonce(nonce: &str) {
    let expires = chrono::Utc::now().timestamp() + NONCE_TTL_SECS * 2;
    NONCE_BLACKLIST.insert(nonce.to_string(), expires);
    NONCE_STORE.remove(nonce);
    let mut nc = NONCE_NC.write().unwrap();
    nc.remove(nonce);
}

pub fn build_www_authenticate(realm: &str, nonce: &str) -> String {
    format!(
        r#"Digest realm="{}", nonce="{}", algorithm=MD5, qop="auth""#,
        realm, nonce
    )
}

pub fn authenticate(
    ctx: &RtspAuthContext,
    authorization: Option<&str>,
    method: &str,
    uri: &str,
    username: &str,
    password: &str,
) -> Result<(), &'static str> {
    match authorization {
        None => Err("missing_authorization"),
        Some(header) => {
            let auth = parse_authorization(header).ok_or("invalid_authorization")?;
            if verify_basic(&auth, username, password) {
                return Ok(());
            }
            if verify_digest(&auth, username, password, method, uri) {
                return Ok(());
            }
            Err("invalid_credentials")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_basic_valid() {
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:secret");
        let auth = parse_authorization(&format!("Basic {}", encoded)).unwrap();
        assert!(verify_basic(&auth, "admin", "secret"));
    }

    #[test]
    fn test_verify_basic_invalid() {
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:wrong");
        let auth = parse_authorization(&format!("Basic {}", encoded)).unwrap();
        assert!(!verify_basic(&auth, "admin", "secret"));
    }

    #[test]
    fn test_verify_digest_valid() {
        let user = "admin";
        let pass = "secret";
        let realm = "RustCam";
        let method = "DESCRIBE";
        let uri = "/live/stream1";
        let (nonce, expires) = generate_nonce();
        let resp =
            compute_digest_response(user, pass, realm, method, uri, &nonce, None, None, None);
        let auth = DigestAuth {
            username: user.to_string(),
            realm: realm.to_string(),
            nonce: nonce.clone(),
            uri: uri.to_string(),
            response: resp,
            algorithm: Some("MD5".to_string()),
            qop: None,
            nc: None,
            cnonce: None,
            opaque: None,
        };
        assert!(verify_digest(
            &AuthHeader::Digest(auth),
            user,
            pass,
            method,
            uri
        ));
        verify_nonce(&nonce, expires);
    }

    #[test]
    fn test_generate_and_verify_nonce() {
        let (nonce, expires) = generate_nonce();
        assert!(verify_nonce(&nonce, expires));
    }

    #[test]
    fn test_authenticate_success() {
        let ctx = RtspAuthContext::new("RustCam")
            .with_defaults(Some("admin".to_string()), Some("secret".to_string()));
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:secret");
        let result = authenticate(
            &ctx,
            Some(&format!("Basic {}", encoded)),
            "DESCRIBE",
            "/live/1",
            "admin",
            "secret",
        );
        assert!(result.is_ok());
    }
}
