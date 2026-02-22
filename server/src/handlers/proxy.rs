use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;

pub async fn proxy_catch_all(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    proxy::proxy_handler(req, body, pool, client).await
}

pub async fn bedrock_invoke(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    proxy::bedrock::bedrock_streaming_handler(req, body, pool, client).await
}
