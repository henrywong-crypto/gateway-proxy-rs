use actix_web::{web, HttpResponse};
use proxy::webfetch::{ApprovalDecision, ApprovalQueue};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn show_intercept_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    approval_queue: web::Data<ApprovalQueue>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let pending_count = proxy::webfetch::list_pending(approval_queue.get_ref(), &session_id).len();
    let html = pages::intercept::render_intercept_view(&session, pending_count);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn show_webfetch_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let html = pages::webfetch::render_webfetch_view(&session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn set_webfetch_intercept_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::set_session_webfetch_intercept(pool.get_ref(), &session_id, true).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/intercept/webfetch", session_id),
        ))
        .finish()
}

pub async fn clear_webfetch_intercept_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::set_session_webfetch_intercept(pool.get_ref(), &session_id, false).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/intercept/webfetch", session_id),
        ))
        .finish()
}

pub async fn set_webfetch_whitelist_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let whitelist = form.get("whitelist").map(|s| s.as_str()).unwrap_or("");
    let whitelist = if whitelist.trim().is_empty() {
        None
    } else {
        Some(whitelist)
    };
    if let Err(e) = db::set_session_webfetch_whitelist(pool.get_ref(), &session_id, whitelist).await
    {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/intercept/webfetch", session_id),
        ))
        .finish()
}

pub async fn clear_webfetch_whitelist_post(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    if let Err(e) = db::set_session_webfetch_whitelist(pool.get_ref(), &session_id, None).await {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/intercept/webfetch", session_id),
        ))
        .finish()
}

pub async fn show_approvals_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    approval_queue: web::Data<ApprovalQueue>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let pending = proxy::webfetch::list_pending(approval_queue.get_ref(), &session_id);
    let html = pages::webfetch::render_approvals_view(&session, &pending);
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn fail_approval_post(
    path: web::Path<(String, String)>,
    approval_queue: web::Data<ApprovalQueue>,
) -> HttpResponse {
    let (session_id, approval_id) = path.into_inner();
    proxy::webfetch::resolve_pending(
        approval_queue.get_ref(),
        &approval_id,
        ApprovalDecision::Fail,
    );
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/intercept/approvals", session_id),
        ))
        .finish()
}

pub async fn mock_approval_post(
    path: web::Path<(String, String)>,
    approval_queue: web::Data<ApprovalQueue>,
) -> HttpResponse {
    let (session_id, approval_id) = path.into_inner();
    proxy::webfetch::resolve_pending(
        approval_queue.get_ref(),
        &approval_id,
        ApprovalDecision::Mock,
    );
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/intercept/approvals", session_id),
        ))
        .finish()
}

pub async fn accept_approval_post(
    path: web::Path<(String, String)>,
    approval_queue: web::Data<ApprovalQueue>,
) -> HttpResponse {
    let (session_id, approval_id) = path.into_inner();
    proxy::webfetch::resolve_pending(
        approval_queue.get_ref(),
        &approval_id,
        ApprovalDecision::Accept,
    );
    HttpResponse::SeeOther()
        .insert_header((
            "Location",
            format!("/_dashboard/sessions/{}/intercept/approvals", session_id),
        ))
        .finish()
}
