mod common;
mod messages;
mod sse;
mod system;
mod tools;
mod webfetch;

use self::common::{build_request_subpage_defs, render_detail_page_content};
pub use self::webfetch::*;
use ::common::models::{ProxyRequest, Session};
use leptos::prelude::*;
use std::collections::HashMap;
use templates::{Breadcrumb, InfoRow, NavLink, Page};

fn build_detail_breadcrumbs(
    session: &Session,
    req: &ProxyRequest,
    current_page: Option<&str>,
) -> Vec<Breadcrumb> {
    let mut crumbs = vec![
        Breadcrumb::link("Home", "/_dashboard"),
        Breadcrumb::link("Sessions", "/_dashboard/sessions"),
        Breadcrumb::link(
            format!("Session {}", session.name),
            format!("/_dashboard/sessions/{}", req.session_id),
        ),
        Breadcrumb::link(
            "Requests",
            format!("/_dashboard/sessions/{}/requests", req.session_id),
        ),
    ];
    if let Some(label) = current_page {
        crumbs.push(Breadcrumb::link(
            format!("Request #{}", req.id),
            format!(
                "/_dashboard/sessions/{}/requests/{}",
                req.session_id, req.id
            ),
        ));
        crumbs.push(Breadcrumb::current(label));
    } else {
        crumbs.push(Breadcrumb::current(format!("Request #{}", req.id)));
    }
    crumbs
}

pub fn render_request_detail_view(req: &ProxyRequest, session: &Session) -> String {
    let base = format!(
        "/_dashboard/sessions/{}/requests/{}",
        req.session_id, req.id
    );

    let subpages = build_request_subpage_defs(req, &base, true);

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{}",
            session.name, req.id
        ),
        breadcrumbs: build_detail_breadcrumbs(session, req, None),
        nav_links: vec![NavLink::back()],
        info_rows: vec![
            InfoRow::new("Method", &req.method),
            InfoRow::new("Path", &req.path),
            InfoRow::new("Model", req.model.as_deref().unwrap_or("")),
            InfoRow::new("Time", &req.timestamp),
        ],
        content: (),
        subpages,
    }
    .render()
}

fn get_page_label(page: &str) -> &str {
    match page {
        "messages" => "Messages",
        "system" => "System",
        "tools" => "Tools",
        "params" => "Params",
        "headers" => "Request Headers",
        "full_json" => "Full JSON",
        "response_headers" => "Response Headers",
        "response_sse" => "Response SSE",
        _ => "Unknown",
    }
}

pub fn render_request_detail_page_view(
    req: &ProxyRequest,
    session: &Session,
    page: &str,
    query: &HashMap<String, String>,
    filters: &[String],
    keep_tool_pairs: i64,
) -> String {
    let page_label = get_page_label(page);

    let base = format!(
        "/_dashboard/sessions/{}/requests/{}",
        req.session_id, req.id
    );

    let detail_page_content =
        render_detail_page_content(req, &base, page, query, filters, keep_tool_pairs);

    let content = view! {
        <h2>{page_label}</h2>
        {detail_page_content.total_view}
        {detail_page_content.controls_view}
        {detail_page_content.content_view}
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - {}",
            session.name, req.id, page_label
        ),
        breadcrumbs: build_detail_breadcrumbs(session, req, Some(page_label)),
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
