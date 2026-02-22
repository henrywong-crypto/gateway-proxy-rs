use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    #[sqlx(try_from = "String")]
    pub id: uuid::Uuid,
    pub name: String,
    pub target_url: String,
    pub tls_verify_disabled: bool,
    pub auth_header: Option<String>,
    pub created_at: Option<String>,
    #[sqlx(default)]
    pub request_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxyRequest {
    #[sqlx(try_from = "String")]
    pub id: uuid::Uuid,
    #[sqlx(try_from = "String")]
    pub session_id: uuid::Uuid,
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
pub struct FilterProfile {
    #[sqlx(try_from = "String")]
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SystemFilter {
    #[sqlx(try_from = "String")]
    pub id: uuid::Uuid,
    #[sqlx(try_from = "String")]
    pub profile_id: uuid::Uuid,
    pub pattern: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ToolFilter {
    #[sqlx(try_from = "String")]
    pub id: uuid::Uuid,
    #[sqlx(try_from = "String")]
    pub profile_id: uuid::Uuid,
    pub name: String,
    pub created_at: Option<String>,
}
