use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn show_webfetch_intercept_page(
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

    let html = pages::detail::render_webfetch_intercept_hub(&request, &session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_webfetch_agent_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String, String)>,
) -> HttpResponse {
    let (session_id, req_id, agent_req_id) = path.into_inner();

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

    let agent_req = match db::get_request(pool.get_ref(), &agent_req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Agent request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::detail::render_webfetch_agent_overview(&request, &session, &agent_req);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_webfetch_agent_subpage(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String, String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let (session_id, req_id, agent_req_id, page) = path.into_inner();

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

    let agent_req = match db::get_request(pool.get_ref(), &agent_req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Agent request not found"),
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
        db::get_filter_profile_keep_tool_pairs(pool.get_ref(), &profile_id)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    let html = pages::detail::render_webfetch_agent_page(
        &request,
        &session,
        &agent_req,
        &page,
        &query,
        &filters,
        keep_tool_pairs,
    );
    HttpResponse::Ok().content_type("text/html").body(html)
}
