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
    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or("server=info,proxy=info"),
    );
    let args = Args::parse();
    let port = args.port;

    let pool = db::init_pool(&args.db).await?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    log::info!("Gateway Proxy listening on http://localhost:{}", port);
    log::info!("Dashboard at http://localhost:{}/_dashboard/", port);

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
            .route(
                "/_dashboard/sessions",
                web::get().to(handlers::sessions_index),
            )
            .route(
                "/_dashboard/sessions/new",
                web::get().to(handlers::new_session),
            )
            .route(
                "/_dashboard/sessions/new",
                web::post().to(handlers::create_session),
            )
            .route(
                "/_dashboard/sessions/{id}",
                web::get().to(handlers::session_show),
            )
            .route(
                "/_dashboard/sessions/{id}/edit",
                web::get().to(handlers::edit_session),
            )
            .route(
                "/_dashboard/sessions/{id}/edit",
                web::post().to(handlers::update_session),
            )
            .route(
                "/_dashboard/filters",
                web::get().to(handlers::filters_index),
            )
            .route(
                "/_dashboard/filters/new",
                web::get().to(handlers::filters_new),
            )
            .route(
                "/_dashboard/filters/new",
                web::post().to(handlers::filters_create),
            )
            .route(
                "/_dashboard/filters/{id}",
                web::get().to(handlers::filter_profile_show),
            )
            .route(
                "/_dashboard/filters/{id}/edit",
                web::get().to(handlers::filter_profile_edit),
            )
            .route(
                "/_dashboard/filters/{id}/edit",
                web::post().to(handlers::filter_profile_update),
            )
            .route(
                "/_dashboard/filters/{id}/activate",
                web::post().to(handlers::filter_profile_activate),
            )
            .route(
                "/_dashboard/filters/{id}/delete",
                web::post().to(handlers::filter_profile_delete),
            )
            .route(
                "/_dashboard/filters/{id}/system",
                web::get().to(handlers::filter_profile_system),
            )
            .route(
                "/_dashboard/filters/{id}/system",
                web::post().to(handlers::filter_profile_system_post),
            )
            .route(
                "/_dashboard/filters/{id}/system/new",
                web::get().to(handlers::filter_profile_system_new),
            )
            .route(
                "/_dashboard/filters/{id}/system/{filter_id}/edit",
                web::get().to(handlers::system_filter_edit),
            )
            .route(
                "/_dashboard/filters/{id}/system/{filter_id}/edit",
                web::post().to(handlers::system_filter_edit_post),
            )
            .route(
                "/_dashboard/filters/{id}/system/{filter_id}/delete",
                web::post().to(handlers::system_filter_delete),
            )
            .route(
                "/_dashboard/filters/{id}/tools",
                web::get().to(handlers::filter_profile_tools),
            )
            .route(
                "/_dashboard/filters/{id}/tools",
                web::post().to(handlers::filter_profile_tools_post),
            )
            .route(
                "/_dashboard/filters/{id}/tools/new",
                web::get().to(handlers::filter_profile_tools_new),
            )
            .route(
                "/_dashboard/filters/{id}/tools/{filter_id}/edit",
                web::get().to(handlers::tool_filter_edit),
            )
            .route(
                "/_dashboard/filters/{id}/tools/{filter_id}/edit",
                web::post().to(handlers::tool_filter_edit_post),
            )
            .route(
                "/_dashboard/filters/{id}/tools/{filter_id}/delete",
                web::post().to(handlers::tool_filter_delete),
            )
            .route(
                "/_dashboard/filters/{id}/messages",
                web::get().to(handlers::filter_profile_messages),
            )
            .route(
                "/_dashboard/filters/{id}/messages",
                web::post().to(handlers::filter_profile_messages_post),
            )
            .route(
                "/_dashboard/sessions/{id}/requests",
                web::get().to(handlers::requests_index),
            )
            .route(
                "/_dashboard/sessions/{id}/requests/{req_id}",
                web::get().to(handlers::request_detail),
            )
            .route(
                "/_dashboard/sessions/{id}/requests/{req_id}/{page}",
                web::get().to(handlers::request_detail_page),
            )
            .route(
                "/_dashboard/sessions/{id}/clear",
                web::post().to(handlers::clear_session_requests),
            )
            .route(
                "/_dashboard/sessions/{id}/delete",
                web::post().to(handlers::delete_session),
            )
            .route(
                "/_proxy/{session_id}/{tail:.*}",
                web::to(handlers::proxy_catch_all),
            )
            .route(
                "/_bedrock/{session_id}/model/{model_id}/invoke-with-response-stream",
                web::post().to(handlers::bedrock_invoke),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
