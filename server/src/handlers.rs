use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::pages;
use crate::Args;

pub async fn home_page(pool: web::Data<SqlitePool>) -> HttpResponse {
    let session_count = db::count_sessions(pool.get_ref()).await.unwrap_or(0);
    let profile_count = db::count_profiles(pool.get_ref()).await.unwrap_or(0);
    let html = pages::home::render_home(session_count, profile_count);
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
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let id = Uuid::new_v4();
    match db::create_session(
        pool.get_ref(),
        &id.to_string(),
        &name,
        &target_url,
        tls_verify_disabled,
        auth_header.as_deref(),
    )
    .await
    {
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

    let profile_id = if let Some(profile_name) = query.get("profile") {
        match db::get_profile_by_name(pool.get_ref(), profile_name).await {
            Ok(Some(p)) => p.id.to_string(),
            _ => db::get_active_profile_id(pool.get_ref())
                .await
                .unwrap_or_default(),
        }
    } else {
        db::get_active_profile_id(pool.get_ref())
            .await
            .unwrap_or_default()
    };

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

    let html = pages::detail::render_detail_page(
        &request,
        &session,
        &page,
        &query,
        &filters,
        keep_tool_pairs,
    );
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
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
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

pub async fn filters_index(pool: web::Data<SqlitePool>) -> HttpResponse {
    let profiles = match db::list_profiles(pool.get_ref()).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let active_profile_id = db::get_active_profile_id(pool.get_ref())
        .await
        .unwrap_or_default();
    let html = pages::filters::render_filters_index(&profiles, &active_profile_id);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filters_new() -> HttpResponse {
    let html = pages::filters::render_new_profile();
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filters_create(
    pool: web::Data<SqlitePool>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let name = match form.get("name") {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return HttpResponse::BadRequest().body("Name is required"),
    };
    match db::create_profile(pool.get_ref(), &name).await {
        Ok(id) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/filters/{}", id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn filter_profile_show(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let active_profile_id = db::get_active_profile_id(pool.get_ref())
        .await
        .unwrap_or_default();
    let system_count = db::count_system_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let tool_count = db::count_tool_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let keep_tool_pairs = db::get_keep_tool_pairs(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let html = pages::filters::render_profile_show(
        &profile,
        &active_profile_id,
        system_count,
        tool_count,
        keep_tool_pairs,
    );
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filter_profile_edit(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_profile_edit(&profile);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filter_profile_update(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let name = match form.get("name") {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return HttpResponse::BadRequest().body("Name is required"),
    };
    if let Err(e) = db::rename_profile(pool.get_ref(), &profile_id, &name).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header(("Location", format!("/_dashboard/filters/{}", profile_id)))
        .finish()
}

pub async fn filter_profile_activate(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let _ = db::set_active_profile_id(pool.get_ref(), &profile_id).await;
    HttpResponse::SeeOther()
        .insert_header(("Location", format!("/_dashboard/filters/{}", profile_id)))
        .finish()
}

pub async fn filter_profile_delete(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let _ = db::delete_profile(pool.get_ref(), &profile_id).await;
    let _ = db::ensure_default_profile(pool.get_ref()).await;
    HttpResponse::SeeOther()
        .insert_header(("Location", "/_dashboard/filters"))
        .finish()
}

pub async fn filter_profile_system(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let system_filters = db::list_system_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_profile_system(&profile, &system_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filter_profile_system_new(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let system_filters = db::list_system_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_profile_system_new(&profile, &system_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filter_profile_system_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    if let Some(pattern) = form.get("pattern") {
        if !pattern.is_empty() {
            let _ = db::add_system_filter(pool.get_ref(), &profile_id, pattern).await;
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/system", profile_id),
        ))
        .finish()
}

pub async fn system_filter_edit_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();

    if let Some(pattern) = form.get("pattern") {
        if !pattern.is_empty() {
            let _ = db::update_system_filter(pool.get_ref(), &filter_id, pattern).await;
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/system", profile_id),
        ))
        .finish()
}

pub async fn system_filter_delete(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    let _ = db::delete_system_filter(pool.get_ref(), &filter_id).await;
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/system", profile_id),
        ))
        .finish()
}

pub async fn filter_profile_tools(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let tool_filters = db::list_tool_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_profile_tools(&profile, &tool_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filter_profile_tools_new(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let tool_filters = db::list_tool_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_profile_tools_new(&profile, &tool_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn system_filter_edit(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let filter = match db::get_system_filter(pool.get_ref(), &filter_id).await {
        Ok(Some(f)) => f,
        Ok(None) => return HttpResponse::NotFound().body("Filter not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_system_filter_edit(&profile, &filter);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn tool_filter_edit(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let filter = match db::get_tool_filter(pool.get_ref(), &filter_id).await {
        Ok(Some(f)) => f,
        Ok(None) => return HttpResponse::NotFound().body("Filter not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_tool_filter_edit(&profile, &filter);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filter_profile_tools_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    if let Some(name) = form.get("name") {
        if !name.is_empty() {
            let _ = db::add_tool_filter(pool.get_ref(), &profile_id, name).await;
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/tools", profile_id),
        ))
        .finish()
}

pub async fn tool_filter_edit_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();

    if let Some(name) = form.get("name") {
        if !name.is_empty() {
            if let Ok(uuid) = uuid::Uuid::parse_str(&filter_id) {
                let _ = db::update_tool_filter(pool.get_ref(), uuid, name).await;
            }
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/tools", profile_id),
        ))
        .finish()
}

pub async fn tool_filter_delete(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    if let Ok(uuid) = uuid::Uuid::parse_str(&filter_id) {
        let _ = db::delete_tool_filter(pool.get_ref(), uuid).await;
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/tools", profile_id),
        ))
        .finish()
}

pub async fn filter_profile_messages(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let keep_tool_pairs = db::get_keep_tool_pairs(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let html = pages::filters::render_profile_messages(&profile, keep_tool_pairs);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filter_profile_messages_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    if let Some(val) = form.get("keep_tool_pairs") {
        if let Ok(n) = val.parse::<i64>() {
            let _ = db::set_message_filter(pool.get_ref(), &profile_id, n).await;
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/messages", profile_id),
        ))
        .finish()
}

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
