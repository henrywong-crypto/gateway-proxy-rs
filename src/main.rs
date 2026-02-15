mod db;
mod models;
mod pages;
mod proxy;
mod truncate;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
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

async fn home_page(pool: web::Data<SqlitePool>) -> HttpResponse {
    match db::list_sessions(pool.get_ref()).await {
        Ok(sessions) => {
            let html = pages::home::render_home(&sessions);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

async fn create_session(
    pool: web::Data<SqlitePool>,
    form: web::Form<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let name = form.get("name").cloned().unwrap_or_default();
    let target_url = form.get("target_url").cloned().unwrap_or_default();

    if name.is_empty() || target_url.is_empty() {
        return HttpResponse::BadRequest().body("Name and target_url are required");
    }

    let id = generate_session_id();
    match db::create_session(pool.get_ref(), &id, &name, &target_url).await {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", format!("/__proxy__/s/{}", id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

async fn session_dashboard(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    args: web::Data<Args>,
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

    let html = pages::dashboard::render_dashboard(&session, &requests, args.port, auto_refresh);
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn request_detail(
    pool: web::Data<SqlitePool>,
    path: web::Path<(String, i64)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let (_session_id, req_id) = path.into_inner();
    let tab = query
        .get("tab")
        .cloned()
        .unwrap_or_else(|| "messages".to_string());

    let request = match db::get_request(pool.get_ref(), req_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().body("Request not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    // If messages tab requested but no messages, fall back to first available
    let effective_tab = if tab == "messages" && request.messages_json.is_none() {
        if request.system_json.is_some() {
            "system"
        } else if request.tools_json.is_some() {
            "tools"
        } else if request.params_json.is_some() {
            "params"
        } else {
            "full_json"
        }
        .to_string()
    } else {
        tab
    };

    let html = pages::detail::render_detail(&request, &effective_tab, &query);
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn clear_session_requests(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let _ = db::clear_requests(pool.get_ref(), &session_id).await;
    HttpResponse::SeeOther()
        .insert_header(("Location", format!("/__proxy__/s/{}", session_id)))
        .finish()
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
        "Dashboard at http://localhost:{}/__proxy__/",
        port
    );

    let args_data = web::Data::new(args);
    let pool_data = web::Data::new(pool);
    let client_data = web::Data::new(client);

    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .app_data(client_data.clone())
            .app_data(args_data.clone())
            .route("/__proxy__/", web::get().to(home_page))
            .route("/__proxy__/sessions", web::post().to(create_session))
            .route("/__proxy__/s/{id}", web::get().to(session_dashboard))
            .route("/__proxy__/s/{id}/r/{req_id}", web::get().to(request_detail))
            .route("/__proxy__/s/{id}/clear", web::post().to(clear_session_requests))
            .route("/s/{session_id}/{tail:.*}", web::to(proxy_catch_all))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
