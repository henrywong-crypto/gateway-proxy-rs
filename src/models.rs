use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub target_url: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxyRequest {
    pub id: i64,
    pub session_id: String,
    pub method: String,
    pub path: String,
    pub timestamp: String,
    pub headers_json: Option<String>,
    pub body_json: Option<String>,
    pub truncated_json: Option<String>,
    pub model: Option<String>,
    pub tools_json: Option<String>,
    pub messages_json: Option<String>,
    pub system_json: Option<String>,
    pub params_json: Option<String>,
    pub note: Option<String>,
    pub created_at: Option<String>,
    pub response_status: Option<i64>,
    pub response_headers_json: Option<String>,
    pub response_body: Option<String>,
    pub response_events_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionWithCount {
    pub id: String,
    pub name: String,
    pub target_url: String,
    pub created_at: Option<String>,
    pub request_count: i64,
}
