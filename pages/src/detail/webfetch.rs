use super::common::{build_request_subpage_defs, count_json_array, render_detail_page_content};
use super::sse::render_response_sse;
use common::models::{ProxyRequest, Session};
use leptos::prelude::*;
use std::collections::HashMap;
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

/// Parse webfetch_rounds_json into a Vec of round objects.
pub fn parse_rounds(json: Option<&str>) -> Option<Vec<serde_json::Value>> {
    json.and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
}

/// Build breadcrumbs for WebFetch Intercept pages.
/// `trail` contains `(label, href)` pairs; the last entry with `None` href is the current page.
fn build_webfetch_breadcrumbs(
    session: &Session,
    req: &ProxyRequest,
    trail: &[(&str, Option<String>)],
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
        Breadcrumb::link(
            format!("Request #{}", req.id),
            format!(
                "/_dashboard/sessions/{}/requests/{}",
                req.session_id, req.id
            ),
        ),
    ];

    for (label, href) in trail {
        match href {
            Some(h) => crumbs.push(Breadcrumb::link(*label, h)),
            None => crumbs.push(Breadcrumb::current(*label)),
        }
    }

    crumbs
}

/// WebFetch Intercept hub — shows intercepted response SSE inline + agent request subpage links.
pub fn render_webfetch_intercept_hub(req: &ProxyRequest, session: &Session) -> String {
    let base = format!(
        "/_dashboard/sessions/{}/requests/{}/webfetch_intercept",
        req.session_id, req.id
    );

    // Intercepted response SSE inline
    let intercepted_view: AnyView = if req.webfetch_first_response_events_json.is_some() {
        let mut sse_req = req.clone();
        sse_req.response_events_json = req.webfetch_first_response_events_json.clone();
        sse_req.response_body = req.webfetch_first_response_body.clone();
        let sse_view = render_response_sse(&sse_req);

        let total_count = count_json_array(req.webfetch_first_response_events_json.as_deref());
        let total_view: AnyView = total_count
            .map(|n| {
                let n = n.to_string();
                view! { <p>"Total: "{n}</p> }.into_any()
            })
            .unwrap_or_else(|| ().into_any());

        view! {
            <h2>"Intercepted Response SSE"</h2>
            {total_view}
            {sse_view}
        }
        .into_any()
    } else {
        ().into_any()
    };

    // Collect all agent_request_ids from all rounds
    let rounds = parse_rounds(req.webfetch_rounds_json.as_deref()).unwrap_or_default();
    let agent_ids: Vec<&str> = rounds
        .iter()
        .flat_map(|r| {
            r.get("agent_request_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .collect();

    let subpages: Vec<Subpage> = agent_ids
        .iter()
        .map(|rid| {
            let short = &rid[..8.min(rid.len())];
            Subpage::new(
                format!("Agent Request #{}", short),
                format!("{}/agent/{}", base, rid),
                String::new(),
            )
        })
        .collect();

    let content = view! {
        {intercepted_view}
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebFetch Intercept",
            session.name, req.id
        ),
        breadcrumbs: build_webfetch_breadcrumbs(session, req, &[("WebFetch Intercept", None)]),
        nav_links: vec![NavLink::back()],
        info_rows: vec![
            InfoRow::new("Method", &req.method),
            InfoRow::new("Path", &req.path),
            InfoRow::new("Model", req.model.as_deref().unwrap_or("")),
            InfoRow::new("Time", &req.timestamp),
        ],
        content,
        subpages,
    }
    .render()
}

/// WebFetch agent request overview — like render_request_detail_view but with webfetch breadcrumbs.
pub fn render_webfetch_agent_overview(
    req: &ProxyRequest,
    session: &Session,
    agent_req: &ProxyRequest,
) -> String {
    let intercept_base = format!(
        "/_dashboard/sessions/{}/requests/{}/webfetch_intercept",
        req.session_id, req.id
    );
    let agent_base = format!("{}/agent/{}", intercept_base, agent_req.id);

    let subpages = build_request_subpage_defs(agent_req, &agent_base, false);

    let short_id = &agent_req.id.to_string()[..8];
    let agent_label = format!("Agent #{}", short_id);

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebFetch Intercept - {}",
            session.name, req.id, agent_label
        ),
        breadcrumbs: build_webfetch_breadcrumbs(
            session,
            req,
            &[
                ("WebFetch Intercept", Some(intercept_base)),
                (&agent_label, None),
            ],
        ),
        nav_links: vec![NavLink::back()],
        info_rows: vec![
            InfoRow::new("Method", &agent_req.method),
            InfoRow::new("Path", &agent_req.path),
            InfoRow::new("Model", agent_req.model.as_deref().unwrap_or("")),
            InfoRow::new("Time", &agent_req.timestamp),
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

/// WebFetch agent request subpage — renders Messages/System/Tools/Params/etc using the same
/// renderers as the main request detail pages, but with webfetch intercept breadcrumbs.
pub fn render_webfetch_agent_page(
    req: &ProxyRequest,
    session: &Session,
    agent_req: &ProxyRequest,
    page: &str,
    query: &HashMap<String, String>,
    filters: &[String],
    keep_tool_pairs: i64,
) -> String {
    let page_label = get_page_label(page);

    let intercept_base = format!(
        "/_dashboard/sessions/{}/requests/{}/webfetch_intercept",
        req.session_id, req.id
    );
    let agent_base = format!("{}/agent/{}", intercept_base, agent_req.id);

    let detail_page_content = render_detail_page_content(
        agent_req,
        &agent_base,
        page,
        query,
        filters,
        keep_tool_pairs,
    );

    let content = view! {
        <h2>{page_label}</h2>
        {detail_page_content.total_view}
        {detail_page_content.controls_view}
        {detail_page_content.content_view}
    };

    let short_id = &agent_req.id.to_string()[..8];
    let agent_label = format!("Agent #{}", short_id);

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebFetch Intercept - {} - {}",
            session.name, req.id, agent_label, page_label
        ),
        breadcrumbs: build_webfetch_breadcrumbs(
            session,
            req,
            &[
                ("WebFetch Intercept", Some(intercept_base)),
                (&agent_label, Some(agent_base)),
                (page_label, None),
            ],
        ),
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
