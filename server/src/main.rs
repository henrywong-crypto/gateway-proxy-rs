mod handlers;

use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
use common::config::AppConfig;

#[derive(Parser, Clone)]
#[command(name = "gateway-proxy-rs")]
pub struct Args {
    #[arg(long, default_value = "8081")]
    pub port: u16,

    #[arg(long, default_value = "proxy.db")]
    pub db: String,

    #[arg(long, default_value = "config.toml")]
    pub config: String,
}

fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/_dashboard", web::get().to(handlers::show_home_page))
        .route(
            "/_dashboard/sessions",
            web::get().to(handlers::show_sessions_page),
        )
        .route(
            "/_dashboard/sessions/new",
            web::get().to(handlers::show_new_session_form),
        )
        .route(
            "/_dashboard/sessions/new",
            web::post().to(handlers::create_session_post),
        )
        .route(
            "/_dashboard/sessions/{id}",
            web::get().to(handlers::show_session_page),
        )
        .route(
            "/_dashboard/sessions/{id}/edit",
            web::get().to(handlers::show_edit_session_form),
        )
        .route(
            "/_dashboard/sessions/{id}/edit",
            web::post().to(handlers::update_session_post),
        )
        .route(
            "/_dashboard/filters",
            web::get().to(handlers::show_filters_page),
        )
        .route(
            "/_dashboard/filters/new",
            web::get().to(handlers::show_new_filter_form),
        )
        .route(
            "/_dashboard/filters/new",
            web::post().to(handlers::create_filter_post),
        )
        .route(
            "/_dashboard/filters/{id}",
            web::get().to(handlers::show_filter_profile_page),
        )
        .route(
            "/_dashboard/filters/{id}/edit",
            web::get().to(handlers::show_edit_filter_profile_form),
        )
        .route(
            "/_dashboard/filters/{id}/edit",
            web::post().to(handlers::update_filter_profile_post),
        )
        .route(
            "/_dashboard/filters/{id}/delete",
            web::post().to(handlers::delete_filter_profile_post),
        )
        .route(
            "/_dashboard/filters/{id}/system",
            web::get().to(handlers::show_system_filters_page),
        )
        .route(
            "/_dashboard/filters/{id}/system",
            web::post().to(handlers::create_system_filter_post),
        )
        .route(
            "/_dashboard/filters/{id}/system/new",
            web::get().to(handlers::show_new_system_filter_form),
        )
        .route(
            "/_dashboard/filters/{id}/system/{filter_id}/edit",
            web::get().to(handlers::show_edit_system_filter_form),
        )
        .route(
            "/_dashboard/filters/{id}/system/{filter_id}/edit",
            web::post().to(handlers::update_system_filter_post),
        )
        .route(
            "/_dashboard/filters/{id}/system/{filter_id}/delete",
            web::post().to(handlers::delete_system_filter_post),
        )
        .route(
            "/_dashboard/filters/{id}/tools",
            web::get().to(handlers::show_tool_filters_page),
        )
        .route(
            "/_dashboard/filters/{id}/tools",
            web::post().to(handlers::create_tool_filter_post),
        )
        .route(
            "/_dashboard/filters/{id}/tools/new",
            web::get().to(handlers::show_new_tool_filter_form),
        )
        .route(
            "/_dashboard/filters/{id}/tools/{filter_id}/edit",
            web::get().to(handlers::show_edit_tool_filter_form),
        )
        .route(
            "/_dashboard/filters/{id}/tools/{filter_id}/edit",
            web::post().to(handlers::update_tool_filter_post),
        )
        .route(
            "/_dashboard/filters/{id}/tools/{filter_id}/delete",
            web::post().to(handlers::delete_tool_filter_post),
        )
        .route(
            "/_dashboard/filters/{id}/messages",
            web::get().to(handlers::show_message_filters_page),
        )
        .route(
            "/_dashboard/filters/{id}/messages",
            web::post().to(handlers::update_message_filters_post),
        )
        .route(
            "/_dashboard/sessions/{id}/requests",
            web::get().to(handlers::show_requests_page),
        )
        .route(
            "/_dashboard/sessions/{id}/requests/{req_id}",
            web::get().to(handlers::show_request_detail_page),
        )
        .route(
            "/_dashboard/sessions/{id}/requests/{req_id}/webfetch_intercept",
            web::get().to(handlers::show_webfetch_intercept_page),
        )
        .route(
            "/_dashboard/sessions/{id}/requests/{req_id}/webfetch_intercept/agent/{agent_req_id}",
            web::get().to(handlers::show_webfetch_agent_page),
        )
        .route(
            "/_dashboard/sessions/{id}/requests/{req_id}/webfetch_intercept/agent/{agent_req_id}/{page}",
            web::get().to(handlers::show_webfetch_agent_subpage),
        )
        .route(
            "/_dashboard/sessions/{id}/requests/{req_id}/{page}",
            web::get().to(handlers::show_request_detail_subpage),
        )
        .route(
            "/_dashboard/sessions/{id}/clear",
            web::post().to(handlers::clear_requests_post),
        )
        .route(
            "/_dashboard/sessions/{id}/delete",
            web::post().to(handlers::delete_session_post),
        )
        .route(
            "/_dashboard/sessions/{id}/error-inject",
            web::get().to(handlers::show_error_inject_page),
        )
        .route(
            "/_dashboard/sessions/{id}/error-inject",
            web::post().to(handlers::set_error_inject_post),
        )
        .route(
            "/_dashboard/sessions/{id}/error-inject/clear",
            web::post().to(handlers::clear_error_inject_post),
        )
        // Tool Intercept hub
        .route(
            "/_dashboard/sessions/{id}/tool-intercept",
            web::get().to(handlers::show_intercept_page),
        )
        // WebFetch Intercept
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/webfetch",
            web::get().to(handlers::show_webfetch_page),
        )
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/webfetch",
            web::post().to(handlers::set_webfetch_intercept_post),
        )
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/webfetch/clear",
            web::post().to(handlers::clear_webfetch_intercept_post),
        )
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/webfetch/whitelist",
            web::post().to(handlers::set_webfetch_whitelist_post),
        )
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/webfetch/whitelist/clear",
            web::post().to(handlers::clear_webfetch_whitelist_post),
        )
        // Pending Approvals
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/approvals",
            web::get().to(handlers::show_approvals_page),
        )
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/approvals/fail/{approval_id}",
            web::post().to(handlers::fail_approval_post),
        )
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/approvals/mock/{approval_id}",
            web::post().to(handlers::mock_approval_post),
        )
        .route(
            "/_dashboard/sessions/{id}/tool-intercept/approvals/accept/{approval_id}",
            web::post().to(handlers::accept_approval_post),
        )
        .route(
            "/_proxy/{session_id}/{tail:.*}",
            web::to(handlers::proxy_catch_all),
        )
        .route(
            "/_bedrock/{session_id}/model/{model_id}/invoke-with-response-stream",
            web::post().to(handlers::bedrock_invoke),
        );
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or("server=info,proxy=info"),
    );
    let args = Args::parse();
    let port = args.port;

    let pool = db::init_pool(&args.db).await?;
    let config = AppConfig::load(&args.config)?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    log::info!("Gateway Proxy listening on http://localhost:{}", port);
    log::info!("Dashboard at http://localhost:{}/_dashboard/", port);

    let args_data = web::Data::new(args);
    let pool_data = web::Data::new(pool);
    let client_data = web::Data::new(client);
    let config_data = web::Data::new(config);
    let approval_queue_data = web::Data::new(proxy::webfetch::new_approval_queue());

    HttpServer::new(move || {
        let payload_cfg = web::PayloadConfig::new(100 * 1024 * 1024); // 100 MB
        App::new()
            .wrap(middleware::NormalizePath::trim())
            .app_data(payload_cfg)
            .app_data(pool_data.clone())
            .app_data(client_data.clone())
            .app_data(args_data.clone())
            .app_data(config_data.clone())
            .app_data(approval_queue_data.clone())
            .configure(configure_routes)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
