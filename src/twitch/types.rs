use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct EventSubMessage {
    pub metadata: EventSubMetadata,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct EventSubMetadata {
    pub message_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionWelcome {
    pub session: Session,
}

#[derive(Debug, Deserialize)]
pub struct Session {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionReconnect {
    pub session: ReconnectSession,
}

#[derive(Debug, Deserialize)]
pub struct ReconnectSession {
    pub reconnect_url: String,
}

#[derive(Debug, Deserialize)]
pub struct StreamOnline {
    #[allow(dead_code)]
    pub broadcaster_user_id: String,
    pub broadcaster_user_login: String,
    pub broadcaster_user_name: String,
}

#[derive(Debug, Deserialize)]
pub struct StreamOffline {
    pub broadcaster_user_login: String,
}

#[derive(Debug, Serialize)]
pub struct CreateSubscription {
    #[serde(rename = "type")]
    pub sub_type: String,
    pub version: String,
    pub condition: serde_json::Value,
    pub transport: Transport,
}

#[derive(Debug, Serialize)]
pub struct Transport {
    pub method: String,
    pub session_id: String,
}
