mod db;
mod models;
mod pages;
mod proxy;
mod truncate;

use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use clap::Parser;
use sqlx::SqlitePool;

#[derive(Parser)]
#[command(name = "gateway-proxy-rs")]
struct Args {
    #[arg(long, default_value = "8081")]
    port: u16,

    #[arg(long, default_value = "proxy.db")]
    db: String,
}

// --- Route handlers ---

async fn home_page() -> HttpResponse {
    let html = pages::home::render_home();
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn sessions_index(pool: web::Data<SqlitePool>) -> HttpResponse {
    match db::list_sessions(pool.get_ref()).await {
        Ok(sessions) => {
            let html = pages::sessions::render_sessions_index(&sessions);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

async fn new_session() -> HttpResponse {
    let html = pages::sessions::render_new_session();
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn create_session(
    pool: web::Data<SqlitePool>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let name = form.get("name").cloned().unwrap_or_default();
    let target_url = form.get("target_url").cloned().unwrap_or_default();
    let tls_verify_disabled = form.get("tls_verify_disabled").map(|v| v == "1").unwrap_or(false);

    if name.is_empty() || target_url.is_empty() {
        return HttpResponse::BadRequest().body("Name and target_url are required");
    }

    let id = generate_session_id();
    match db::create_session(pool.get_ref(), &id, &name, &target_url, tls_verify_disabled).await {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/sessions/{}", id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

async fn session_show(
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

async fn requests_index(
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

async fn request_detail(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, i64)>,
) -> HttpResponse {
    let (session_id, req_id) = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let request = match db::get_request(pool.get_ref(), req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::detail::render_detail_overview(&request, &session);
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn request_detail_page(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, i64, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let (session_id, req_id, page) = path.into_inner();

    let session = match db::get_session(pool.get_ref(), &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().body("Session not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let request = match db::get_request(pool.get_ref(), req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let html = pages::detail::render_detail_page(&request, &session, &page, &query);
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn clear_session_requests(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let _ = db::clear_requests(pool.get_ref(), &session_id).await;
    HttpResponse::SeeOther()
        .insert_header(("Location", format!("/_dashboard/sessions/{}/requests", session_id)))
        .finish()
}

async fn delete_session(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let _ = db::delete_session(pool.get_ref(), &session_id).await;
    HttpResponse::SeeOther()
        .insert_header(("Location", "/_dashboard/sessions"))
        .finish()
}

async fn edit_session(
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

async fn update_session(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let name = form.get("name").cloned().unwrap_or_default();
    let target_url = form.get("target_url").cloned().unwrap_or_default();
    let tls_verify_disabled = form.get("tls_verify_disabled").map(|v| v == "1").unwrap_or(false);

    if name.is_empty() || target_url.is_empty() {
        return HttpResponse::BadRequest().body("Name and target_url are required");
    }

    match db::update_session(pool.get_ref(), &session_id, &name, &target_url, tls_verify_disabled).await {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/_dashboard/sessions/{}", session_id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

async fn proxy_catch_all(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> HttpResponse {
    proxy::proxy_handler(req, body, pool, client).await
}

fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..12).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let port = args.port;

    let pool = db::init_pool(&args.db)
        .await
        .expect("Failed to initialize database");

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client");

    println!(
        "Gateway Proxy listening on http://localhost:{}",
        port
    );
    println!(
        "Dashboard at http://localhost:{}/_dashboard/",
        port
    );

    let args_data = web::Data::new(args);
    let pool_data = web::Data::new(pool);
    let client_data = web::Data::new(client);

    HttpServer::new(move || {
        let payload_cfg = web::PayloadConfig::new(100 * 1024 * 1024); // 100 MB
        App::new()
            .wrap(middleware::NormalizePath::trim())
            .app_data(payload_cfg)
            .app_data(pool_data.clone())
            .app_data(client_data.clone())
            .app_data(args_data.clone())
            .route("/_dashboard", web::get().to(home_page))
            .route("/_dashboard/sessions", web::get().to(sessions_index))
            .route("/_dashboard/sessions/new", web::get().to(new_session))
            .route("/_dashboard/sessions/new", web::post().to(create_session))
            .route("/_dashboard/sessions/{id}", web::get().to(session_show))
            .route("/_dashboard/sessions/{id}/edit", web::get().to(edit_session))
            .route("/_dashboard/sessions/{id}/edit", web::post().to(update_session))
            .route("/_dashboard/sessions/{id}/requests", web::get().to(requests_index))
            .route("/_dashboard/sessions/{id}/requests/{req_id}", web::get().to(request_detail))
            .route("/_dashboard/sessions/{id}/requests/{req_id}/{page}", web::get().to(request_detail_page))
            .route("/_dashboard/sessions/{id}/clear", web::post().to(clear_session_requests))
            .route("/_dashboard/sessions/{id}/delete", web::post().to(delete_session))
            .route("/_proxy/{session_id}/{tail:.*}", web::to(proxy_catch_all))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
