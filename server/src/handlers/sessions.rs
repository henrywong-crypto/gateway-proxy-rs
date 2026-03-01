use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;
use templates::Pagination;
use uuid::Uuid;

use crate::Args;

pub async fn show_home_page(pool: web::Data<SqlitePool>) -> HttpResponse {
    let session_count = db::count_sessions(pool.get_ref()).await.unwrap_or(0);
    let profile_count = db::count_filter_profiles(pool.get_ref()).await.unwrap_or(0);
    let html = pages::home::render_home_view(session_count, profile_count);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_sessions_page(
    pool: web::Data<SqlitePool>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let page: i64 = query
        .get("page")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page: i64 = 50;

    let total = match db::count_sessions(pool.get_ref()).await {
        Ok(n) => n,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let offset = (page - 1) * per_page;
    let sessions = match db::list_sessions_paginated(pool.get_ref(), per_page, offset).await {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let pagination = Pagination::new(page, total, per_page, "/_dashboard/sessions", "");
    let html = pages::sessions::render_sessions_view(&sessions, &pagination);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_new_session_form(pool: web::Data<SqlitePool>) -> HttpResponse {
    let profiles = db::list_filter_profiles(pool.get_ref())
        .await
        .unwrap_or_default();
    let default_profile_id = db::get_default_filter_profile_id(pool.get_ref())
        .await
        .unwrap_or_default();
    let html = pages::sessions::render_new_session_form(&profiles, &default_profile_id);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn create_session_post(
    pool: web::Data<SqlitePool>,
    form: web::Form<HashMap<String, String>>,
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
    let x_api_key = form.get("x_api_key").and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let profile_id = form.get("profile_id").and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let id = Uuid::new_v4();
    let id_str = id.to_string();
    match db::create_session(
        pool.get_ref(),
        &db::SessionParams {
            id: &id_str,
            name: &name,
            target_url: &target_url,
            tls_verify_disabled,
            auth_header: auth_header.as_deref(),
            x_api_key: x_api_key.as_deref(),
            profile_id: profile_id.as_deref(),
        },
    )
    .await
    {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/sessions/{}", id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn show_session_page(
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

    let profile_name = if let Some(ref pid) = session.profile_id {
        match db::get_filter_profile(pool.get_ref(), pid).await {
            Ok(Some(p)) => Some(p.name),
            _ => None,
        }
    } else {
        None
    };

    let html =
        pages::session_show::render_session_view(&session, args.port, profile_name.as_deref());
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_edit_session_form(
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

    let profiles = db::list_filter_profiles(pool.get_ref())
        .await
        .unwrap_or_default();
    let html = pages::sessions::render_edit_session_form(&session, args.port, &profiles);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn update_session_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
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
    let x_api_key = form.get("x_api_key").and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let profile_id = form.get("profile_id").and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    match db::update_session(
        pool.get_ref(),
        &db::SessionParams {
            id: &session_id,
            name: &name,
            target_url: &target_url,
            tls_verify_disabled,
            auth_header: auth_header.as_deref(),
            x_api_key: x_api_key.as_deref(),
            profile_id: profile_id.as_deref(),
        },
    )
    .await
    {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/sessions/{}", session_id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn delete_session_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::delete_session(pool.get_ref(), &session_id).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header(("Location", "/_dashboard/sessions"))
        .finish()
}
