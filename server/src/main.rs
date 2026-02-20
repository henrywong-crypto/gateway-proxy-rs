mod handlers;
mod pages;

use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;

#[derive(Parser, Clone)]
#[command(name = "gateway-proxy-rs")]
pub struct Args {
    #[arg(long, default_value = "8081")]
    pub port: u16,

    #[arg(long, default_value = "proxy.db")]
    pub db: String,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let port = args.port;

    let pool = db::init_pool(&args.db).await?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

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
            .route("/_dashboard", web::get().to(handlers::home_page))
            .route("/_dashboard/sessions", web::get().to(handlers::sessions_index))
            .route("/_dashboard/sessions/new", web::get().to(handlers::new_session))
            .route("/_dashboard/sessions/new", web::post().to(handlers::create_session))
            .route("/_dashboard/sessions/{id}", web::get().to(handlers::session_show))
            .route("/_dashboard/sessions/{id}/edit", web::get().to(handlers::edit_session))
            .route("/_dashboard/sessions/{id}/edit", web::post().to(handlers::update_session))
            .route("/_dashboard/sessions/{id}/requests", web::get().to(handlers::requests_index))
            .route("/_dashboard/sessions/{id}/requests/{req_id}", web::get().to(handlers::request_detail))
            .route("/_dashboard/sessions/{id}/requests/{req_id}/{page}", web::get().to(handlers::request_detail_page))
            .route("/_dashboard/sessions/{id}/clear", web::post().to(handlers::clear_session_requests))
            .route("/_dashboard/sessions/{id}/delete", web::post().to(handlers::delete_session))
            .route("/_proxy/{session_id}/{tail:.*}", web::to(handlers::proxy_catch_all))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
