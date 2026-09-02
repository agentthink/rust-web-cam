use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static GB_NONCE_STORE: Lazy<DashMap<String, i64>> = Lazy::new(|| DashMap::new());
const NONCE_TTL_SECS: i64 = 600;

#[derive(Debug, Clone, PartialEq)]
pub enum VerifyResult {
    Valid,
    InvalidCredentials,
}

#[derive(Debug, Clone)]
pub struct SipAuthParams {
    pub username: String,
    pub realm: String,
    pub nonce: String,
    pub uri: String,
    pub response: String,
    pub algorithm: Option<String>,
    pub qop: Option<String>,
    pub nc: Option<String>,
    pub cnonce: Option<String>,
}

pub fn parse_sip_authorization(header: &str) -> Option<SipAuthParams> {
    let header = header.trim();
    if !header.starts_with("Digest ") {
        return None;
    }
    let rest = header.trim_start_matches("Digest ").trim();
    let params = parse_digest_params(rest)?;

    Some(SipAuthParams {
        username: params.get("username")?.clone(),
        realm: params.get("realm").cloned().unwrap_or_default(),
        nonce: params.get("nonce")?.clone(),
        uri: params.get("uri")?.clone(),
        response: params.get("response")?.clone(),
        algorithm: params.get("algorithm").cloned(),
        qop: params.get("qop").cloned(),
        nc: params.get("nc").cloned(),
        cnonce: params.get("cnonce").cloned(),
    })
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

pub fn verify_sip_digest(params: &SipAuthParams, password: &str, method: &str) -> VerifyResult {
    if params.username.is_empty() {
        return VerifyResult::InvalidCredentials;
    }

    let ha1 = format!(
        "{:x}",
        md5::compute(format!("{}:{}:{}", params.username, params.realm, password))
    );
    let ha2 = format!("{:x}", md5::compute(format!("{}:{}", method, params.uri)));

    let expected = if let Some(qop) = &params.qop {
        let nc = params.nc.as_deref().unwrap_or("00000001");
        let cnonce = params.cnonce.as_deref().unwrap_or("0");
        format!(
            "{:x}",
            md5::compute(format!(
                "{}:{}:{}:{}:{}:{}",
                ha1, params.nonce, nc, cnonce, qop, ha2
            ))
        )
    } else {
        format!(
            "{:x}",
            md5::compute(format!("{}:{}:{}", ha1, params.nonce, ha2))
        )
    };

    if params.response == expected {
        VerifyResult::Valid
    } else {
        VerifyResult::InvalidCredentials
    }
}

pub fn generate_nonce() -> String {
    let nonce = uuid::Uuid::new_v4().to_string();
    let expires = chrono::Utc::now().timestamp() + NONCE_TTL_SECS;
    GB_NONCE_STORE.insert(nonce.clone(), expires);
    nonce
}

pub fn verify_nonce(nonce: &str) -> bool {
    let now = chrono::Utc::now().timestamp();
    match GB_NONCE_STORE.get(nonce) {
        Some(entry) => {
            if *entry < now {
                GB_NONCE_STORE.remove(nonce);
                false
            } else {
                true
            }
        }
        None => false,
    }
}

pub fn consume_nonce(nonce: &str) -> bool {
    GB_NONCE_STORE.remove(nonce).is_some()
}

pub fn cleanup_expired_nonces() {
    let now = chrono::Utc::now().timestamp();
    GB_NONCE_STORE.retain(|_, expiry| *expiry > now);
}

pub fn build_www_authenticate(realm: &str, nonce: &str) -> String {
    format!(r#"Digest realm="{}", nonce="{}", qop="auth""#, realm, nonce)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sip_authorization() {
        let header = r#"Digest username="34020000001110000001", realm="34020000", nonce="abc123", uri="sip:34020000001110000001", response="xyz""#;
        let params = parse_sip_authorization(header).unwrap();
        assert_eq!(params.username, "34020000001110000001");
        assert_eq!(params.nonce, "abc123");
        assert_eq!(params.response, "xyz");
    }

    #[test]
    fn test_verify_sip_digest() {
        let password = "password123";
        let method = "REGISTER";
        let uri = "sip:34020000001110000001";
        let realm = "34020000";
        let nonce = "abc123";

        let ha1 = format!(
            "{:x}",
            md5::compute(format!("34020000001110000001:{}:{}", realm, password))
        );
        let ha2 = format!("{:x}", md5::compute(format!("{}:{}", method, uri)));
        let expected = format!("{:x}", md5::compute(format!("{}:{}:{}", ha1, nonce, ha2)));

        let params = SipAuthParams {
            username: "34020000001110000001".to_string(),
            realm: realm.to_string(),
            nonce: nonce.to_string(),
            uri: uri.to_string(),
            response: expected,
            algorithm: None,
            qop: None,
            nc: None,
            cnonce: None,
        };
        assert_eq!(
            verify_sip_digest(&params, password, method),
            VerifyResult::Valid
        );
    }

    #[test]
    fn test_nonce_lifecycle() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();
        assert!(verify_nonce(&nonce1));
        assert!(!verify_nonce("nonexistent"));
        assert!(verify_nonce(&nonce1));
        assert!(verify_nonce(&nonce2));
        consume_nonce(&nonce1);
        assert!(!verify_nonce(&nonce1));
        assert!(verify_nonce(&nonce2));
        consume_nonce(&nonce2);
        assert!(!verify_nonce(&nonce2));
    }
}
