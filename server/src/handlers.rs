use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;

use crate::pages;
use crate::Args;

pub async fn home_page() -> HttpResponse {
    let html = pages::home::render_home();
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn sessions_index(pool: web::Data<SqlitePool>) -> HttpResponse {
    match db::list_sessions(pool.get_ref()).await {
        Ok(sessions) => {
            let html = pages::sessions::render_sessions_index(&sessions);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn new_session() -> HttpResponse {
    let html = pages::sessions::render_new_session();
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn create_session(
    pool: web::Data<SqlitePool>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let (name, target_url) = match (form.get("name"), form.get("target_url")) {
        (Some(n), Some(t)) if !n.is_empty() && !t.is_empty() => (n.clone(), t.clone()),
        _ => return HttpResponse::BadRequest().body("Name and target_url are required"),
    };
    let tls_verify_disabled = form.get("tls_verify_disabled").is_some_and(|v| v == "1");
    let auth_header = form.get("auth_header").and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    });

    let id = db::generate_id();
    match db::create_session(pool.get_ref(), &id, &name, &target_url, tls_verify_disabled, auth_header.as_deref()).await {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/sessions/{}", id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn session_show(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    args: web::Data<Args>,
) -> HttpResponse {
    let session_id = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::session_show::render_session_show(&session, args.port);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn requests_index(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let auto_refresh = query.get("refresh").map(|v| v.as_str()) == Some("on");

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let requests = match db::list_requests(pool.get_ref(), &session_id).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::requests::render_requests_index(&session, &requests, auto_refresh);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn request_detail(
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

    let html = pages::detail::render_detail_overview(&request, &session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn request_detail_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
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

    let filters: Vec<String> = if page == "system" {
        db::list_system_filters(pool.get_ref())
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|f| f.pattern)
            .collect()
    } else {
        Vec::new()
    };

    let html = pages::detail::render_detail_page(&request, &session, &page, &query, &filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn clear_session_requests(
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

pub async fn delete_session(pool: web::Data<SqlitePool>, path: web::Path<String>) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::delete_session(pool.get_ref(), &session_id).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header(("Location", "/_dashboard/sessions"))
        .finish()
}

pub async fn edit_session(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    args: web::Data<Args>,
) -> HttpResponse {
    let session_id = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::sessions::render_edit_session(&session, args.port);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn update_session(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let (name, target_url) = match (form.get("name"), form.get("target_url")) {
        (Some(n), Some(t)) if !n.is_empty() && !t.is_empty() => (n.clone(), t.clone()),
        _ => return HttpResponse::BadRequest().body("Name and target_url are required"),
    };
    let tls_verify_disabled = form.get("tls_verify_disabled").is_some_and(|v| v == "1");
    let auth_header = form.get("auth_header").and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    });

    match db::update_session(
        pool.get_ref(),
        &session_id,
        &name,
        &target_url,
        tls_verify_disabled,
        auth_header.as_deref(),
    )
    .await
    {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/sessions/{}", session_id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn filters_page(
    pool: web::Data<SqlitePool>,
) -> HttpResponse {
    match db::list_system_filters(pool.get_ref()).await {
        Ok(filters) => {
            let html = pages::filters::render_filters_page(&filters);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn filters_post(
    pool: web::Data<SqlitePool>,
    form: web::Form<std::collections::HashMap<String, String>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    if let Some(delete_id) = query.get("delete") {
        let _ = db::delete_system_filter(pool.get_ref(), delete_id).await;
    } else if let Some(edit_id) = query.get("edit") {
        if let Some(pattern) = form.get("pattern") {
            if !pattern.is_empty() {
                let _ = db::update_system_filter(pool.get_ref(), edit_id, pattern).await;
            }
        }
    } else if let Some(pattern) = form.get("pattern") {
        if !pattern.is_empty() {
            let _ = db::add_system_filter(pool.get_ref(), pattern).await;
        }
    }
    HttpResponse::SeeOther()
        .insert_header(("Location", "/_dashboard/filters"))
        .finish()
}

pub async fn proxy_catch_all(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> HttpResponse {
    proxy::proxy_handler(req, body, pool, client).await
}
