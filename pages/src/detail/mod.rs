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

pub fn render_request_detail_view(
    req: &ProxyRequest,
    session: &Session,
    prev_id: Option<&str>,
    next_id: Option<&str>,
) -> String {
    let base = format!(
        "/_dashboard/sessions/{}/requests/{}",
        req.session_id, req.id
    );

    let subpages = build_request_subpage_defs(req, &base, true);

    let mut nav_links = vec![];
    if let Some(id) = prev_id {
        let href = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, id);
        nav_links.push(NavLink::new("← Newer", href));
    }
    if let Some(id) = next_id {
        let href = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, id);
        nav_links.push(NavLink::new("Older →", href));
    }
    nav_links.push(NavLink::back());

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{}",
            session.name, req.id
        ),
        breadcrumbs: build_detail_breadcrumbs(session, req, None),
        nav_links,
        info_rows: vec![
            InfoRow::new("Method", &req.method),
            InfoRow::new("Path", &req.path),
            InfoRow::new("Model", req.model.as_deref().unwrap_or("")),
            InfoRow::new("Time", req.created_at.get(11..19).unwrap_or(&req.created_at)),
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
    prev_id: Option<&str>,
    next_id: Option<&str>,
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

    let mut nav_links = vec![];
    if let Some(id) = prev_id {
        let href = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, id);
        nav_links.push(NavLink::new("← Newer", href));
    }
    if let Some(id) = next_id {
        let href = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, id);
        nav_links.push(NavLink::new("Older →", href));
    }
    nav_links.push(NavLink::back());

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - {}",
            session.name, req.id, page_label
        ),
        breadcrumbs: build_detail_breadcrumbs(session, req, Some(page_label)),
        nav_links,
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
