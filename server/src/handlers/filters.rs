use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn show_filters_page(pool: web::Data<SqlitePool>) -> HttpResponse {
    let profiles = match db::list_filter_profiles(pool.get_ref()).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_filters_view(&profiles);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_new_filter_form() -> HttpResponse {
    let html = pages::filters::render_new_profile_form();
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn create_filter_post(
    pool: web::Data<SqlitePool>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let name = match form.get("name") {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return HttpResponse::BadRequest().body("Name is required"),
    };
    match db::create_filter_profile(pool.get_ref(), &name).await {
        Ok(id) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/filters/{}", id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn show_filter_profile_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
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
    let keep_tool_pairs = db::get_filter_profile_keep_tool_pairs(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let html =
        pages::filters::render_profile_view(&profile, system_count, tool_count, keep_tool_pairs);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_edit_filter_profile_form(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_edit_profile_form(&profile);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn update_filter_profile_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let name = match form.get("name") {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return HttpResponse::BadRequest().body("Name is required"),
    };
    if let Err(e) = db::set_filter_profile_name(pool.get_ref(), &profile_id, &name).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header(("Location", format!("/_dashboard/filters/{}", profile_id)))
        .finish()
}

pub async fn delete_filter_profile_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    // Protect the default profile from deletion
    match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) if p.is_default => {
            return HttpResponse::BadRequest().body("Cannot delete the default profile");
        }
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
        _ => {}
    }

    if let Err(e) = db::delete_filter_profile(pool.get_ref(), &profile_id).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header(("Location", "/_dashboard/filters"))
        .finish()
}

pub async fn show_system_filters_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let system_filters = db::list_system_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_system_filters_view(&profile, &system_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_new_system_filter_form(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let system_filters = db::list_system_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_new_system_filter_form(&profile, &system_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn create_system_filter_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    if let Some(pattern) = form.get("pattern") {
        if !pattern.is_empty() {
            if let Err(e) = db::create_system_filter(pool.get_ref(), &profile_id, pattern).await {
                return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
            }
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/system", profile_id),
        ))
        .finish()
}

pub async fn show_edit_system_filter_form(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let filter = match db::get_system_filter(pool.get_ref(), &filter_id).await {
        Ok(Some(f)) => f,
        Ok(None) => return HttpResponse::NotFound().body("Filter not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_edit_system_filter_form(&profile, &filter);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn update_system_filter_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();

    if let Some(pattern) = form.get("pattern") {
        if !pattern.is_empty() {
            if let Err(e) = db::update_system_filter(pool.get_ref(), &filter_id, pattern).await {
                return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
            }
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/system", profile_id),
        ))
        .finish()
}

pub async fn delete_system_filter_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    if let Err(e) = db::delete_system_filter(pool.get_ref(), &filter_id).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/system", profile_id),
        ))
        .finish()
}

pub async fn show_tool_filters_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let tool_filters = db::list_tool_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_tool_filters_view(&profile, &tool_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_new_tool_filter_form(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let tool_filters = db::list_tool_filters(pool.get_ref(), &profile_id)
        .await
        .unwrap_or_default();
    let html = pages::filters::render_new_tool_filter_form(&profile, &tool_filters);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn create_tool_filter_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    if let Some(name) = form.get("name") {
        if !name.is_empty() {
            if let Err(e) = db::create_tool_filter(pool.get_ref(), &profile_id, name).await {
                return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
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

pub async fn show_edit_tool_filter_form(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let filter = match db::get_tool_filter(pool.get_ref(), &filter_id).await {
        Ok(Some(f)) => f,
        Ok(None) => return HttpResponse::NotFound().body("Filter not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::filters::render_edit_tool_filter_form(&profile, &filter);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn update_tool_filter_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();

    if let Some(name) = form.get("name") {
        if !name.is_empty() {
            if let Err(e) = db::update_tool_filter(pool.get_ref(), &filter_id, name).await {
                return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
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

pub async fn delete_tool_filter_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (profile_id, filter_id) = path.into_inner();
    if let Err(e) = db::delete_tool_filter(pool.get_ref(), &filter_id).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/tools", profile_id),
        ))
        .finish()
}

pub async fn show_message_filters_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let profile_id = path.into_inner();
    let profile = match db::get_filter_profile(pool.get_ref(), &profile_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Profile not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let keep_tool_pairs = db::get_filter_profile_keep_tool_pairs(pool.get_ref(), &profile_id)
        .await
        .unwrap_or(0);
    let html = pages::filters::render_message_filters_view(&profile, keep_tool_pairs);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn update_message_filters_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let profile_id = path.into_inner();

    if let Some(val) = form.get("keep_tool_pairs") {
        if let Ok(n) = val.parse::<i64>() {
            if let Err(e) =
                db::set_filter_profile_message_filter(pool.get_ref(), &profile_id, n).await
            {
                return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
            }
        }
    }

    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/filters/{}/messages", profile_id),
        ))
        .finish()
}
