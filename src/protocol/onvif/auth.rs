use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use sha1::{Digest, Sha1};

const PASSWORD_DIGEST_URI: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest";
const PASSWORD_TEXT_URI: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText";
const TIMESTAMP_TOLERANCE_SECS: i64 = 300;

static NONCE_STORE: Lazy<DashMap<String, i64>> = Lazy::new(|| DashMap::new());

pub struct UsernameToken {
    pub username: String,
    pub password: String,
    pub password_type: PasswordType,
    pub nonce: String,
    pub created: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PasswordType {
    Digest,
    Text,
    Unknown,
}

impl Default for PasswordType {
    fn default() -> Self {
        PasswordType::Digest
    }
}

impl UsernameToken {
    pub fn from_xml(soap_body: &[u8]) -> Option<Self> {
        let username = Self::extract_tag(soap_body, b"Username")
            .or_else(|| Self::extract_tag(soap_body, b"wsse:Username"))?;
        let password = Self::extract_tag(soap_body, b"Password")
            .or_else(|| Self::extract_tag(soap_body, b"wsse:Password"))
            .unwrap_or_default();
        let password_type = Self::extract_password_type(soap_body);
        let nonce = Self::extract_tag(soap_body, b"Nonce")
            .or_else(|| Self::extract_tag(soap_body, b"wsse:Nonce"))
            .unwrap_or_default();
        let created = Self::extract_tag(soap_body, b"Created")
            .or_else(|| Self::extract_tag(soap_body, b"wsse:Created"))
            .unwrap_or_default();
        Some(Self {
            username,
            password,
            password_type,
            nonce,
            created,
        })
    }

    fn extract_password_type(soap_body: &[u8]) -> PasswordType {
        let content = String::from_utf8_lossy(soap_body);
        if content.contains(PASSWORD_DIGEST_URI) {
            return PasswordType::Digest;
        }
        if content.contains(PASSWORD_TEXT_URI) {
            return PasswordType::Text;
        }
        PasswordType::Unknown
    }

    fn extract_tag(data: &[u8], tag: &[u8]) -> Option<String> {
        let tag_str = String::from_utf8_lossy(tag);
        let local = tag_str.split(':').last().unwrap_or(&tag_str);
        let content = String::from_utf8_lossy(data);

        let open_tag = format!("<{}", local);
        let close_tag = format!("</{}>", local);

        if let Some(start) = content.find(&open_tag) {
            let after_open = &content[start..];
            if let Some(gt) = after_open.find('>') {
                let value_start = start + gt + 1;
                if let Some(end) = content[value_start..].find(&close_tag) {
                    let value = &content[value_start..value_start + end];
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }

        if tag_str.contains(':') {
            let prefixed_open = format!("<{}", tag_str);
            let prefixed_close = format!("</{}>", tag_str);
            if let Some(start) = content.find(&prefixed_open) {
                let after_open = &content[start..];
                if let Some(gt) = after_open.find('>') {
                    let value_start = start + gt + 1;
                    if let Some(end) = content[value_start..].find(&prefixed_close) {
                        let value = &content[value_start..value_start + end];
                        let trimmed = value.trim();
                        if !trimmed.is_empty() {
                            return Some(trimmed.to_string());
                        }
                    }
                }
            }
        } else {
            let search_pattern = format!(":{}>", local);
            if let Some(prefix_pos) = content.find(&search_pattern) {
                let open_tag_start = content[..prefix_pos].rfind('<')?;
                let close_tag_start = prefix_pos + search_pattern.len();
                let value_start = close_tag_start;
                let prefixed_close =
                    format!("</{}:{}>", &content[open_tag_start + 1..prefix_pos], local);
                if let Some(end) = content[value_start..].find(&prefixed_close) {
                    let value = &content[value_start..value_start + end];
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }

        None
    }

    pub fn verify(&self, stored_password: &str) -> bool {
        if self.username.is_empty() || stored_password.is_empty() {
            return false;
        }
        if self.nonce.is_empty() || self.created.is_empty() {
            return false;
        }
        if self.password_type != PasswordType::Digest {
            return false;
        }

        if !Self::validate_timestamp(&self.created) {
            return false;
        }
        if !Self::check_and_mark_nonce(&self.nonce) {
            return false;
        }

        let nonce_decoded = match STANDARD.decode(&self.nonce) {
            Ok(n) => n,
            Err(_) => return false,
        };
        let mut hasher = Sha1::new();
        hasher.update(&nonce_decoded);
        hasher.update(self.created.as_bytes());
        hasher.update(stored_password.as_bytes());
        let result = hasher.finalize();
        let expected = STANDARD.encode(result);
        self.password == expected
    }

    fn validate_timestamp(created: &str) -> bool {
        match DateTime::parse_from_rfc3339(created) {
            Ok(dt) => {
                let now = Utc::now();
                let diff = (now - dt.with_timezone(&Utc)).num_seconds().abs();
                diff <= TIMESTAMP_TOLERANCE_SECS
            }
            Err(_) => false,
        }
    }

    fn check_and_mark_nonce(nonce: &str) -> bool {
        Self::cleanup_expired_nonces();
        if NONCE_STORE.contains_key(nonce) {
            return false;
        }
        let expiry = Utc::now().timestamp() + TIMESTAMP_TOLERANCE_SECS * 2;
        NONCE_STORE.insert(nonce.to_string(), expiry);
        true
    }

    fn cleanup_expired_nonces() {
        let now = Utc::now().timestamp();
        NONCE_STORE.retain(|_, expiry| *expiry > now);
    }

    pub fn build_digest(_username: &str, password: &str) -> (String, String, String) {
        let mut nonce_bytes = [0u8; 16];
        getrandom::getrandom(&mut nonce_bytes).expect("failed to get random bytes");
        let nonce_b64 = STANDARD.encode(nonce_bytes);
        let created = chrono::Utc::now().to_rfc3339();
        let mut hasher = Sha1::new();
        hasher.update(&nonce_bytes);
        hasher.update(created.as_bytes());
        hasher.update(password.as_bytes());
        let digest = STANDARD.encode(hasher.finalize());
        (nonce_b64, created, digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag() {
        let xml = br#"<wsse:Username>admin</wsse:Username>"#;
        let val = UsernameToken::extract_tag(xml, b"Username");
        assert_eq!(val, Some("admin".to_string()));
    }

    #[test]
    fn test_build_digest() {
        let (nonce, created, digest) = UsernameToken::build_digest("admin", "password");
        assert!(!nonce.is_empty());
        assert!(!created.is_empty());
        assert!(!digest.is_empty());
    }

    #[test]
    fn test_verify_accepts_valid_digest() {
        let pwd_str = "admin123";
        let (nonce, created, _) = UsernameToken::build_digest("admin", pwd_str);
        let expected_nonce_decoded = STANDARD.decode(&nonce).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(&expected_nonce_decoded);
        hasher.update(created.as_bytes());
        hasher.update(pwd_str.as_bytes());
        let digest = STANDARD.encode(hasher.finalize());
        let t = UsernameToken {
            username: "admin".to_string(),
            password: digest,
            password_type: PasswordType::Digest,
            nonce,
            created,
        };
        assert!(t.verify(pwd_str));
    }

    #[test]
    fn test_verify_rejects_reused_nonce() {
        let pwd = "admin123";
        let (nonce, created, _) = UsernameToken::build_digest("admin", pwd);
        let (_, _, digest) = UsernameToken::build_digest("admin", pwd);
        let t1 = UsernameToken {
            username: "admin".to_string(),
            password: "any".to_string(),
            password_type: PasswordType::Digest,
            nonce: nonce.clone(),
            created: created.clone(),
        };
        let _ = t1.verify(pwd);
        let t2 = UsernameToken {
            username: "admin".to_string(),
            password: "any".to_string(),
            password_type: PasswordType::Digest,
            nonce,
            created,
        };
        assert!(!t2.verify(pwd));
    }
}
