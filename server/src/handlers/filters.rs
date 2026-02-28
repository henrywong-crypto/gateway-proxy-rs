use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::pages;

pub async fn filters_index(pool: web::Data<SqlitePool>) -> HttpResponse {
    let profiles = match db::list_profiles(pool.get_ref()).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_filters_index(&profiles);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filters_new() -> HttpResponse {
    let html = pages::filters::render_new_profile();
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn filters_create(
    pool: web::Data<SqlitePool>,
    form: web::Form<HashMap<String, String>>,
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
    let system_count = db::count_system_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let tool_count = db::count_tool_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let keep_tool_pairs = db::get_keep_tool_pairs(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let html =
        pages::filters::render_profile_show(&profile, system_count, tool_count, keep_tool_pairs);
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
    form: web::Form<HashMap<String, String>>,
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

pub async fn filter_profile_delete(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    // Protect the default profile from deletion
    match db::get_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) if p.is_default => {
            return HttpResponse::BadRequest().body("Cannot delete the default profile");
        }
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
        _ => {}
    }

    let _ = db::delete_profile(pool.get_ref(), &profile_id).await;
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
    form: web::Form<HashMap<String, String>>,
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

pub async fn system_filter_edit_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
    form: web::Form<HashMap<String, String>>,
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

pub async fn filter_profile_tools_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
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

pub async fn tool_filter_edit_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();

    if let Some(name) = form.get("name") {
        if !name.is_empty() {
            let _ = db::update_tool_filter(pool.get_ref(), &filter_id, name).await;
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
    let _ = db::delete_tool_filter(pool.get_ref(), &filter_id).await;
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
    form: web::Form<HashMap<String, String>>,
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
