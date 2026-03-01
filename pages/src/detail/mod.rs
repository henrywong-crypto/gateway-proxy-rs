mod common;
mod messages;
mod sse;
mod system;
mod tools;
mod webfetch;

pub use self::webfetch::*;
use self::{
    common::{render_kv_table, render_response_headers},
    messages::render_messages,
    sse::render_response_sse,
    system::render_system,
    tools::render_tools,
};
use ::common::models::{ProxyRequest, Session};
use leptos::prelude::*;
use std::collections::HashMap;
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

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
    let req = req.clone();

    let base = format!(
        "/_dashboard/sessions/{}/requests/{}",
        req.session_id, req.id
    );
    let has_response = req.response_body.is_some() || req.response_events_json.is_some();

    fn count_json_array(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.as_array().map(|a| a.len()))
    }

    fn count_json_object(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.as_object().map(|o| o.len()))
    }

    let has_ws = req.webfetch_first_response_events_json.is_some()
        || req.webfetch_followup_body_json.is_some();

    let subpage_defs: Vec<(&str, &str, bool, String)> = vec![
        (
            "messages",
            "Messages",
            req.messages_json.is_some(),
            count_json_array(req.messages_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "system",
            "System",
            req.system_json.is_some(),
            count_json_array(req.system_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "tools",
            "Tools",
            req.tools_json.is_some(),
            count_json_array(req.tools_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "params",
            "Params",
            req.params_json.is_some(),
            count_json_object(req.params_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        ("full_json", "Full JSON", true, String::new()),
        (
            "response_sse",
            "Response SSE",
            req.response_events_json.is_some(),
            count_json_array(req.response_events_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "headers",
            "Request Headers",
            true,
            count_json_object(req.headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "response_headers",
            "Response Headers",
            has_response,
            count_json_object(req.response_headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "webfetch_intercept",
            "WebFetch Intercept",
            has_ws,
            String::new(),
        ),
    ];

    let subpages: Vec<Subpage> = subpage_defs
        .into_iter()
        .filter(|(_, _, available, _)| *available)
        .map(|(key, label, _, count)| Subpage::new(label, format!("{}/{}", base, key), count))
        .collect();

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{}",
            session.name, req.id
        ),
        breadcrumbs: build_detail_breadcrumbs(session, &req, None),
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

pub fn render_request_detail_page_view(
    req: &ProxyRequest,
    session: &Session,
    page: &str,
    query: &HashMap<String, String>,
    filters: &[String],
    keep_tool_pairs: i64,
) -> String {
    let req = req.clone();
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

    let truncate = query.get("truncate").map(|v| v.as_str()) != Some("off");
    let order = query
        .get("order")
        .cloned()
        .unwrap_or_else(|| "desc".to_string());

    let base = format!(
        "/_dashboard/sessions/{}/requests/{}",
        req.session_id, req.id
    );

    let mut controls_view: AnyView = ().into_any();

    let content_view: AnyView = match page {
        "messages" => {
            if let Some(ref json_str) = req.messages_json {
                let toggle_order = if order == "desc" { "asc" } else { "desc" };
                let toggle_href = format!("{}/messages?order={}", base, toggle_order);
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
        "system" => req
            .system_json
            .as_deref()
            .map(|s| render_system(s, filters))
            .unwrap_or_else(|| view! { <p>"No system prompt."</p> }.into_any()),
        "tools" => req
            .tools_json
            .as_deref()
            .map(|s| render_tools(s, filters))
            .unwrap_or_else(|| view! { <p>"No tools."</p> }.into_any()),
        "params" => req
            .params_json
            .as_deref()
            .map(render_kv_table)
            .unwrap_or_else(|| view! { <p>"No params."</p> }.into_any()),
        "headers" => {
            let h = req.headers_json.as_deref().unwrap_or("{}");
            render_kv_table(h)
        }
        "full_json" => {
            let json = if truncate {
                req.truncated_json
                    .as_deref()
                    .or(req.note.as_deref())
                    .unwrap_or("")
            } else {
                req.body_json
                    .as_deref()
                    .or(req.note.as_deref())
                    .unwrap_or("")
            };
            let toggle_href = format!(
                "{}/full_json?truncate={}",
                base,
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
        "response_headers" => render_response_headers(&req),
        "response_sse" => render_response_sse(&req),
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
        "messages" => count_json_items(req.messages_json.as_deref()),
        "system" => count_json_items(req.system_json.as_deref()),
        "tools" => count_json_items(req.tools_json.as_deref()),
        "params" => count_json_items(req.params_json.as_deref()),
        "headers" => count_json_items(req.headers_json.as_deref()),
        "response_headers" => count_json_items(req.response_headers_json.as_deref()),
        "response_sse" => count_json_items(req.response_events_json.as_deref()),
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

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - {}",
            session.name, req.id, page_label
        ),
        breadcrumbs: build_detail_breadcrumbs(session, &req, Some(page_label)),
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
