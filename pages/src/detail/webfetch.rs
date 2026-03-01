use super::{
    common::{render_kv_table, render_response_headers},
    messages::render_messages,
    sse::render_response_sse,
    system::render_system,
    tools::render_tools,
};
use common::models::{ProxyRequest, Session};
use leptos::prelude::*;
use std::collections::HashMap;
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

fn count_json_array(json: Option<&str>) -> Option<usize> {
    json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v.as_array().map(|a| a.len()))
}

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

    fn count_json_array(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.as_array().map(|a| a.len()))
    }

    fn count_json_object(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.as_object().map(|o| o.len()))
    }

    let has_response =
        agent_req.response_body.is_some() || agent_req.response_events_json.is_some();

    let subpage_defs: Vec<(&str, &str, bool, String)> = vec![
        (
            "messages",
            "Messages",
            agent_req.messages_json.is_some(),
            count_json_array(agent_req.messages_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "system",
            "System",
            agent_req.system_json.is_some(),
            count_json_array(agent_req.system_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "tools",
            "Tools",
            agent_req.tools_json.is_some(),
            count_json_array(agent_req.tools_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "params",
            "Params",
            agent_req.params_json.is_some(),
            count_json_object(agent_req.params_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        ("full_json", "Full JSON", true, String::new()),
        (
            "response_sse",
            "Response SSE",
            agent_req.response_events_json.is_some(),
            count_json_array(agent_req.response_events_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "headers",
            "Request Headers",
            true,
            count_json_object(agent_req.headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "response_headers",
            "Response Headers",
            has_response,
            count_json_object(agent_req.response_headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
    ];

    let subpages: Vec<Subpage> = subpage_defs
        .into_iter()
        .filter(|(_, _, available, _)| *available)
        .map(|(key, label, _, count)| Subpage::new(label, format!("{}/{}", agent_base, key), count))
        .collect();

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
    let page_label = match page {
        "messages" => "Messages",
        "system" => "System",
        "tools" => "Tools",
        "params" => "Params",
        "headers" => "Request Headers",
        "full_json" => "Full JSON",
        "response_headers" => "Response Headers",
        "response_sse" => "Response SSE",
        _ => "Unknown",
    };

    let intercept_base = format!(
        "/_dashboard/sessions/{}/requests/{}/webfetch_intercept",
        req.session_id, req.id
    );
    let agent_base = format!("{}/agent/{}", intercept_base, agent_req.id);

    let truncate = query.get("truncate").map(|v| v.as_str()) != Some("off");
    let order = query
        .get("order")
        .cloned()
        .unwrap_or_else(|| "desc".to_string());

    let mut controls_view: AnyView = ().into_any();

    let content_view: AnyView = match page {
        "messages" => {
            if let Some(ref json_str) = agent_req.messages_json {
                let toggle_order = if order == "desc" { "asc" } else { "desc" };
                let toggle_href = format!("{}/messages?order={}", agent_base, toggle_order);
                let showing = if order == "desc" {
                    "newest first"
                } else {
                    "oldest first"
                };
                let switch_to = if order == "desc" {
                    "oldest first"
                } else {
                    "newest first"
                };
                controls_view = view! {
                    <div>"Showing: "{showing}" | "<a href={toggle_href}>"Switch to "{switch_to}</a></div>
                }
                .into_any();
                render_messages(json_str, &order, keep_tool_pairs)
            } else {
                view! { <p>"No messages."</p> }.into_any()
            }
        }
        "system" => agent_req
            .system_json
            .as_deref()
            .map(|s| render_system(s, filters))
            .unwrap_or_else(|| view! { <p>"No system prompt."</p> }.into_any()),
        "tools" => agent_req
            .tools_json
            .as_deref()
            .map(|s| render_tools(s, filters))
            .unwrap_or_else(|| view! { <p>"No tools."</p> }.into_any()),
        "params" => agent_req
            .params_json
            .as_deref()
            .map(render_kv_table)
            .unwrap_or_else(|| view! { <p>"No params."</p> }.into_any()),
        "headers" => {
            let h = agent_req.headers_json.as_deref().unwrap_or("{}");
            render_kv_table(h)
        }
        "full_json" => {
            let json = if truncate {
                agent_req
                    .truncated_json
                    .as_deref()
                    .or(agent_req.note.as_deref())
                    .unwrap_or("")
            } else {
                agent_req
                    .body_json
                    .as_deref()
                    .or(agent_req.note.as_deref())
                    .unwrap_or("")
            };
            let toggle_href = format!(
                "{}/full_json?truncate={}",
                agent_base,
                if truncate { "off" } else { "on" }
            );
            let toggle_label = if truncate {
                "Show full strings"
            } else {
                "Show truncated"
            };
            controls_view = view! {
                <a href={toggle_href}>{toggle_label}</a>
            }
            .into_any();
            let json = json.to_string();
            view! {
                <textarea readonly rows="30" cols="80" wrap="off">{json}</textarea>
            }
            .into_any()
        }
        "response_headers" => render_response_headers(agent_req),
        "response_sse" => render_response_sse(agent_req),
        _ => view! { <p>"Unknown tab"</p> }.into_any(),
    };

    fn count_json_items(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| {
                v.as_array()
                    .map(|a| a.len())
                    .or_else(|| v.as_object().map(|o| o.len()))
            })
    }

    let total_count = match page {
        "messages" => count_json_items(agent_req.messages_json.as_deref()),
        "system" => count_json_items(agent_req.system_json.as_deref()),
        "tools" => count_json_items(agent_req.tools_json.as_deref()),
        "params" => count_json_items(agent_req.params_json.as_deref()),
        "headers" => count_json_items(agent_req.headers_json.as_deref()),
        "response_headers" => count_json_items(agent_req.response_headers_json.as_deref()),
        "response_sse" => count_json_items(agent_req.response_events_json.as_deref()),
        _ => None,
    };
    let total_view: AnyView = total_count
        .map(|n| {
            let n = n.to_string();
            view! { <p>"Total: "{n}</p> }.into_any()
        })
        .unwrap_or_else(|| ().into_any());

    let content = view! {
        <h2>{page_label}</h2>
        {total_view}
        {controls_view}
        {content_view}
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
