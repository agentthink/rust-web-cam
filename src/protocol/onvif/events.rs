use chrono::Utc;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct OnvifEvent {
    pub topic: String,
    pub utc_time: String,
    pub source: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Preset {
    pub token: String,
    pub name: String,
}

pub struct PullPointServer {
    subscriptions: std::sync::Mutex<Vec<PullPointSubscription>>,
    presets: std::sync::Mutex<Vec<Preset>>,
}

struct PullPointSubscription {
    subscription_ref: String,
    messages: VecDeque<OnvifEvent>,
    expires_at: i64,
}

impl PullPointServer {
    pub fn new() -> Self {
        Self { subscriptions: std::sync::Mutex::new(Vec::new()), presets: std::sync::Mutex::new(Vec::new()) }
    }

    pub fn get_presets(&self) -> Vec<Preset> { self.presets.lock().unwrap().clone() }

    pub fn save_preset(&self, name: String) -> String {
        let token = format!("preset_{}", uuid::Uuid::new_v4());
        self.presets.lock().unwrap().push(Preset { token: token.clone(), name });
        token
    }

    pub fn remove_preset(&self, token: &str) {
        self.presets.lock().unwrap().retain(|p| p.token != token);
    }

    pub fn build_get_presets_response(presets: &[Preset]) -> String {
        let presets_xml: String = presets.iter()
            .map(|p| format!(r#"<PTZPreset token="{}"><Name>{}</Name></PTZPreset>"#, p.token, p.name))
            .collect();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body xmlns:wsdl="http://www.onvif.org/ver10/ptz/wsdl">
<GetPresetsResponse xmlns="http://www.onvif.org/ver10/ptz/wsdl">{}</GetPresetsResponse>
</s:Body></s:Envelope>"#,
            presets_xml
        )
    }

    pub fn create_subscription_with_timeout(&self, timeout_hours: i64) -> String {
        let subscription_ref = format!("urn:uuid:{}", uuid::Uuid::new_v4());
        let expires_at = Utc::now().timestamp() + (timeout_hours * 3600);
        self.subscriptions.lock().unwrap().push(PullPointSubscription {
            subscription_ref: subscription_ref.clone(), messages: VecDeque::new(), expires_at,
        });
        subscription_ref
    }

    pub fn pull_messages(&self, subscription_ref: &str, max_messages: usize, _timeout_secs: u32) -> Vec<OnvifEvent> {
        let now = Utc::now().timestamp();
        let mut subs = self.subscriptions.lock().unwrap();
        if let Some(idx) = subs.iter().position(|s| s.subscription_ref == subscription_ref) {
            if subs[idx].expires_at < now { subs.remove(idx); return Vec::new(); }
            let mut results = Vec::new();
            for _ in 0..max_messages {
                if let Some(msg) = subs[idx].messages.pop_front() { results.push(msg); } else { break; }
            }
            return results;
        }
        Vec::new()
    }

    pub fn renew_subscription(&self, subscription_ref: &str, timeout_hours: i64) -> Option<String> {
        let mut subs = self.subscriptions.lock().unwrap();
        if let Some(sub) = subs.iter_mut().find(|s| s.subscription_ref == subscription_ref) {
            sub.expires_at = Utc::now().timestamp() + (timeout_hours * 3600);
            return Some(Utc::now().to_rfc3339());
        }
        None
    }

    pub fn unsubscribe(&self, subscription_ref: &str) -> bool {
        let len_before = self.subscriptions.lock().unwrap().len();
        self.subscriptions.lock().unwrap().retain(|s| s.subscription_ref != subscription_ref);
        self.subscriptions.lock().unwrap().len() < len_before
    }

    pub fn build_create_subscription_response(subscription_ref: &str) -> String {
        let now = Utc::now().to_rfc3339();
        let termination = Utc::now().checked_add_signed(chrono::Duration::hours(24)).map(|t| t.to_rfc3339()).unwrap_or(now.clone());
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body>
<wsnt:CreatePullPointSubscriptionResponse xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2">
  <wsnt:SubscriptionReference>{}</wsnt:SubscriptionReference>
  <wsnt:CurrentTime>{}</wsnt:CurrentTime>
  <wsnt:TerminationTime>{}</wsnt:TerminationTime>
</wsnt:CreatePullPointSubscriptionResponse>
</s:Body></s:Envelope>"#,
            subscription_ref, now, termination
        )
    }

    pub fn build_pull_messages_response(messages: &[OnvifEvent], _subscription_ref: &str) -> String {
        let now = Utc::now().to_rfc3339();
        let messages_xml: String = messages.iter().map(|m| format!("<wsnt:NotificationMessage><wsnt:Topic>{}</wsnt:Topic></wsnt:NotificationMessage>", m.topic)).collect();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body>
<wsnt:PullMessagesResponse xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2">
  {messages_xml}
  <wsnt:CurrentTime>{now}</wsnt:CurrentTime>
</wsnt:PullMessagesResponse>
</s:Body></s:Envelope>"#
        )
    }

    pub fn build_get_event_properties_response() -> String {
        let now = Utc::now().to_rfc3339();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body>
<GetEventPropertiesResponse xmlns="http://www.onvif.org/ver10/events/wsdl">
  <wsnt:TopicSet xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2">
    <wsnt:Topic name="Device/Status/StateChange"/>
    <wsnt:Topic name="VideoSource/MotionAlarm"/>
  </wsnt:TopicSet>
  <tt:CurrentTime xmlns:tt="http://www.onvif.org/ver10/schema">{now}</tt:CurrentTime>
</GetEventPropertiesResponse>
</s:Body></s:Envelope>"#
        )
    }

    pub fn build_renew_response(termination: &str) -> String {
        let now = Utc::now().to_rfc3339();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body>
<wsnt:RenewResponse xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2">
  <wsnt:CurrentTime>{now}</wsnt:CurrentTime>
  <wsnt:TerminationTime>{termination}</wsnt:TerminationTime>
</wsnt:RenewResponse>
</s:Body></s:Envelope>"#
        )
    }

    pub fn build_unsubscribe_response() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body><wsnt:UnsubscribeResponse xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"/></s:Body>
</s:Envelope>"#.to_string()
    }
}

impl Default for PullPointServer {
    fn default() -> Self { Self::new() }
}