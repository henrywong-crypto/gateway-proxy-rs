use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::pages;

pub async fn websearch_detail_hub(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (session_id, req_id) = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let request = match db::get_request(pool.get_ref(), &req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::detail::render_websearch_hub(&request, &session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn websearch_first_response(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (session_id, req_id) = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let request = match db::get_request(pool.get_ref(), &req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::detail::render_websearch_first_response(&request, &session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn websearch_followup_hub(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (session_id, req_id) = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let request = match db::get_request(pool.get_ref(), &req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::detail::render_websearch_followup_hub(&request, &session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn websearch_followup_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let (session_id, req_id, page) = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let request = match db::get_request(pool.get_ref(), &req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let profile_id = session.profile_id.clone().unwrap_or_default();

    let filters: Vec<String> = match page.as_str() {
        "system" => db::list_system_filters(pool.get_ref(), &profile_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|f| f.pattern)
            .collect(),
        "tools" => db::list_tool_filters(pool.get_ref(), &profile_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|f| f.name)
            .collect(),
        _ => Vec::new(),
    };

    let keep_tool_pairs = if page == "messages" {
        db::get_keep_tool_pairs(pool.get_ref(), &profile_id)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    let html = pages::detail::render_websearch_followup_page(
        &request,
        &session,
        &page,
        &query,
        &filters,
        keep_tool_pairs,
    );
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn websearch_round(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String, String)>,
) -> HttpResponse {
    let (session_id, req_id, round_idx_str) = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let request = match db::get_request(pool.get_ref(), &req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let round_idx: usize = match round_idx_str.parse() {
        Ok(i) => i,
        Err(_) => return HttpResponse::BadRequest().body("Invalid round index"),
    };

    let html = pages::detail::render_websearch_round(&request, &session, round_idx);
    HttpResponse::Ok().content_type("text/html").body(html)
}
