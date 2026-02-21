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

    let make_table = |items: Vec<(&str, &str, bool)>| -> String {
        let rows: String = items
            .into_iter()
            .filter(|(_, _, available)| *available)
            .map(|(key, label, _)| {
                format!(
                    r#"<tr><td><a href="{}/{}">{}</a></td></tr>"#,
                    base,
                    key,
                    html_escape(label)
                )
            })
            .collect();
        format!("<table>{}</table>", rows)
    };

    let request_links = make_table(vec![
        ("messages", "Messages", req.messages_json.is_some()),
        ("system", "System", req.system_json.is_some()),
        ("tools", "Tools", req.tools_json.is_some()),
        ("params", "Params", req.params_json.is_some()),
        ("full_json", "Full JSON", true),
    ]);

    let headers_links = make_table(vec![
        ("headers", "Request Headers", true),
        ("response_headers", "Response Headers", has_response),
    ]);

    let response_links = make_table(vec![(
        "response_sse",
        "SSE",
        req.response_events_json.is_some(),
    )]);

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
        <h2>"Request"</h2>
        <div inner_html={request_links}/>
        <h2>"Response"</h2>
        <div inner_html={response_links}/>
        <h2>"Headers"</h2>
        <div inner_html={headers_links}/>
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
            .map(render_tools)
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

    let body = view! {
        <div inner_html={bc}/>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>{page_label}</h2>
        <div inner_html={controls_html}/>
        <div inner_html={content_html}/>
    };

    page_layout(&title, body.to_html())
}
