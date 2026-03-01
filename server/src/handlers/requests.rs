use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;
use templates::Pagination;

pub async fn show_requests_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let auto_refresh = query.get("refresh").map(|v| v.as_str()) == Some("on");
    let page: i64 = query
        .get("page")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page: i64 = 50;

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let total = match db::count_requests(pool.get_ref(), &session_id).await {
        Ok(n) => n,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let offset = (page - 1) * per_page;
    let requests =
        match db::list_requests_paginated(pool.get_ref(), &session_id, per_page, offset).await {
            Ok(r) => r,
            Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
        };

    let base_url = format!("/_dashboard/sessions/{}/requests", session_id);
    let extra_params = if auto_refresh {
        "&refresh=on".to_string()
    } else {
        String::new()
    };
    let pagination = Pagination::new(page, total, per_page, &base_url, &extra_params);

    let html =
        pages::requests::render_requests_view(&session, &requests, auto_refresh, &pagination);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_request_detail_page(
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

    let html = pages::detail::render_request_detail_view(&request, &session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_request_detail_subpage(
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

    // Use the session's profile_id for filters
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

    let html = pages::detail::render_request_detail_page_view(
        &request,
        &session,
        &page,
        &query,
        &filters,
        keep_tool_pairs,
    );
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn clear_requests_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::clear_requests(pool.get_ref(), &session_id).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/requests", session_id),
        ))
        .finish()
}
