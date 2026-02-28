use actix_web::{web, HttpRequest, HttpResponse};
use proxy::websearch::ApprovalQueue;
use sqlx::SqlitePool;

pub async fn proxy_catch_all(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
    approval_queue: web::Data<ApprovalQueue>,
) -> Result<HttpResponse, actix_web::Error> {
    proxy::proxy_handler(req, body, pool, client, approval_queue).await
}

pub async fn bedrock_invoke(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    proxy::bedrock::bedrock_streaming_handler(req, body, pool, client).await
}
