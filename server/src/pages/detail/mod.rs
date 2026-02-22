mod common;
mod messages;
mod sse;
mod system;
mod tools;

use leptos::prelude::*;

use self::common::{breadcrumb_html, render_kv_table, render_response_headers};
use self::messages::render_messages;
use self::sse::render_response_sse;
use self::system::render_system;
use self::tools::render_tools;
use crate::pages::{html_escape, page_layout};
use ::common::models::{ProxyRequest, Session};

pub fn render_detail_overview(req: &ProxyRequest, session: &Session) -> String {
    let req = req.clone();
    let title = format!(
        "Gateway Proxy - Session {} - Request #{}",
        session.name, req.id
    );

    let method = html_escape(&req.method);
    let path = html_escape(&req.path);
    let model = req.model.as_deref().map(html_escape).unwrap_or_default();
    let timestamp = html_escape(&req.timestamp);

    let base = format!(
        "/_dashboard/sessions/{}/requests/{}",
        req.session_id, req.id
    );
    let has_response = req.response_body.is_some() || req.response_events_json.is_some();

    fn json_array_len(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.as_array().map(|a| a.len()))
    }

    fn json_object_len(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.as_object().map(|o| o.len()))
    }

    let subpages: Vec<(&str, &str, bool, String)> = vec![
        (
            "messages",
            "Messages",
            req.messages_json.is_some(),
            json_array_len(req.messages_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "system",
            "System",
            req.system_json.is_some(),
            json_array_len(req.system_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "tools",
            "Tools",
            req.tools_json.is_some(),
            json_array_len(req.tools_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "params",
            "Params",
            req.params_json.is_some(),
            json_object_len(req.params_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        ("full_json", "Full JSON", true, String::new()),
        (
            "response_sse",
            "Response SSE",
            req.response_events_json.is_some(),
            json_array_len(req.response_events_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "headers",
            "Request Headers",
            true,
            json_object_len(req.headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "response_headers",
            "Response Headers",
            has_response,
            json_object_len(req.response_headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
    ];

    let subpages: Vec<_> = subpages
        .into_iter()
        .filter(|(_, _, available, _)| *available)
        .collect();

    let subpages_html: String = subpages
        .iter()
        .map(|(key, label, _, count)| {
            format!(
                r#"<tr><td><a href="{}/{}">{}</a></td><td>{}</td></tr>"#,
                base,
                key,
                html_escape(label),
                count,
            )
        })
        .collect();

    let subpages_table = format!(
        r#"<table><tr><th>Page</th><th>Count</th></tr>{}</table>"#,
        subpages_html
    );

    let bc = breadcrumb_html(session, &req, None);

    let body = view! {
        <div inner_html={bc}/>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>"Info"</h2>
        <table>
            <tr><td>"Method"</td><td>{method}</td></tr>
            <tr><td>"Path"</td><td>{path}</td></tr>
            <tr><td>"Model"</td><td>{model}</td></tr>
            <tr><td>"Time"</td><td>{timestamp}</td></tr>
        </table>
        <h2>"Subpages"</h2>
        <div inner_html={subpages_table}/>
    };

    page_layout(&title, body.to_html())
}

pub fn render_detail_page(
    req: &ProxyRequest,
    session: &Session,
    page: &str,
    query: &std::collections::HashMap<String, String>,
    filters: &[String],
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
    let title = format!(
        "Gateway Proxy - Session {} - Request #{} - {}",
        session.name, req.id, page_label
    );

    let truncate = query.get("truncate").map(|v| v.as_str()) != Some("off");
    let order = query
        .get("order")
        .cloned()
        .unwrap_or_else(|| "desc".to_string());

    let base = format!(
        "/_dashboard/sessions/{}/requests/{}",
        req.session_id, req.id
    );

    let mut controls_html = String::new();

    let content_html = match page {
        "messages" => {
            if let Some(ref json_str) = req.messages_json {
                let toggle_order = if order == "desc" { "asc" } else { "desc" };
                let toggle_href = format!("{}/messages?order={}", base, toggle_order);
                controls_html = format!(
                    r#"<div>Showing: {} | <a href="{}">Switch to {}</a></div>"#,
                    if order == "desc" {
                        "newest first"
                    } else {
                        "oldest first"
                    },
                    toggle_href,
                    if order == "desc" {
                        "oldest first"
                    } else {
                        "newest first"
                    }
                );
                render_messages(json_str, &order)
            } else {
                "<p>No messages.</p>".to_string()
            }
        }
        "system" => req
            .system_json
            .as_deref()
            .map(|s| render_system(s, filters))
            .unwrap_or_else(|| "<p>No system prompt.</p>".to_string()),
        "tools" => req
            .tools_json
            .as_deref()
            .map(|s| render_tools(s, filters))
            .unwrap_or_else(|| "<p>No tools.</p>".to_string()),
        "params" => req
            .params_json
            .as_deref()
            .map(render_kv_table)
            .unwrap_or_else(|| "<p>No params.</p>".to_string()),
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
            controls_html = format!(r#"<a href="{}">{}</a>"#, toggle_href, toggle_label,);
            format!(
                r#"<textarea readonly rows="30" cols="80" wrap="off">{}</textarea>"#,
                html_escape(json)
            )
        }
        "response_headers" => render_response_headers(&req),
        "response_sse" => render_response_sse(&req),
        _ => "<p>Unknown tab</p>".to_string(),
    };

    let bc = breadcrumb_html(session, &req, Some((page, page_label)));

    fn json_count(json: Option<&str>) -> Option<usize> {
        json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| {
                v.as_array()
                    .map(|a| a.len())
                    .or_else(|| v.as_object().map(|o| o.len()))
            })
    }

    let total_count = match page {
        "messages" => json_count(req.messages_json.as_deref()),
        "system" => json_count(req.system_json.as_deref()),
        "tools" => json_count(req.tools_json.as_deref()),
        "params" => json_count(req.params_json.as_deref()),
        "headers" => json_count(req.headers_json.as_deref()),
        "response_headers" => json_count(req.response_headers_json.as_deref()),
        "response_sse" => json_count(req.response_events_json.as_deref()),
        _ => None,
    };
    let total_html = total_count
        .map(|n| format!("<p>Total: {}</p>", n))
        .unwrap_or_default();

    let body = view! {
        <div inner_html={bc}/>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>{page_label}</h2>
        <div inner_html={total_html}/>
        <div inner_html={controls_html}/>
        <div inner_html={content_html}/>
    };

    page_layout(&title, body.to_html())
}
