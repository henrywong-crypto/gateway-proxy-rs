use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn show_error_inject_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::error_inject::render_error_inject_view(&session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn set_error_inject_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let error_type = form.get("error_type").map(|s| s.as_str()).unwrap_or("");
    if let Err(e) =
        db::set_session_error_inject(pool.get_ref(), &session_id, Some(error_type)).await
    {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/error-inject", session_id),
        ))
        .finish()
}

pub async fn clear_error_inject_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::set_session_error_inject(pool.get_ref(), &session_id, None).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/error-inject", session_id),
        ))
        .finish()
}
