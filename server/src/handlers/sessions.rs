use actix_web::{web, HttpResponse};
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

pub async fn new_session(pool: web::Data<SqlitePool>) -> HttpResponse {
    let profiles = db::list_profiles(pool.get_ref()).await.unwrap_or_default();
    let default_profile_id = db::get_default_profile_id(pool.get_ref())
        .await
        .unwrap_or_default();
    let html = pages::sessions::render_new_session(&profiles, &default_profile_id);
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

    let profile_name = if let Some(ref pid) = session.profile_id {
        match db::get_profile(pool.get_ref(), pid).await {
            Ok(Some(p)) => Some(p.name),
            _ => None,
        }
    } else {
        None
    };

    let html =
        pages::session_show::render_session_show(&session, args.port, profile_name.as_deref());
    HttpResponse::Ok().content_type("text/html").body(html)
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

    let profiles = db::list_profiles(pool.get_ref()).await.unwrap_or_default();
    let html = pages::sessions::render_edit_session(&session, args.port, &profiles);
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

pub async fn delete_session(pool: web::Data<SqlitePool>, path: web::Path<String>) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::delete_session(pool.get_ref(), &session_id).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header(("Location", "/_dashboard/sessions"))
        .finish()
}
