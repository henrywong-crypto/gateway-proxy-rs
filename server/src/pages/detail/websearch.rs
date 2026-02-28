use super::{
    common::render_kv_table, messages::render_messages, sse::render_response_sse,
    system::render_system, tools::render_tools,
};
use ::common::models::{ProxyRequest, Session};
use leptos::prelude::*;
use std::collections::HashMap;
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

use crate::pages::html_escape;

/// Build breadcrumbs for WebSearch pages.
/// `trail` contains `(label, href)` pairs; the last entry with `None` href is the current page.
fn websearch_breadcrumbs(
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

fn json_array_len(json: Option<&str>) -> Option<usize> {
    json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v.as_array().map(|a| a.len()))
}

/// WebSearch Hub page — lists per-round subpages.
pub fn render_websearch_hub(req: &ProxyRequest, session: &Session) -> String {
    let base = format!(
        "/_dashboard/sessions/{}/requests/{}/websearch",
        req.session_id, req.id
    );

    let rounds_count = parse_rounds(req.ws_rounds_json.as_deref())
        .map(|r| r.len())
        .unwrap_or(0);

    // Always show at least 1 round if we have interception data
    let display_count = if rounds_count > 0 {
        rounds_count
    } else if req.ws_first_response_events_json.is_some() || req.ws_followup_body_json.is_some() {
        1
    } else {
        0
    };

    let subpages: Vec<Subpage> = (0..display_count)
        .map(|i| {
            Subpage::new(
                format!("Round {}", i + 1),
                format!("{}/rounds/{}", base, i),
                String::new(),
            )
        })
        .collect();

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebSearch",
            session.name, req.id
        ),
        breadcrumbs: websearch_breadcrumbs(session, req, &[("WebSearch", None)]),
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

/// WebSearch Intercepted Response SSE — SSE events table (reuses render_response_sse).
pub fn render_websearch_first_response(req: &ProxyRequest, session: &Session) -> String {
    let mut sse_req = req.clone();
    sse_req.response_events_json = req.ws_first_response_events_json.clone();
    sse_req.response_body = req.ws_first_response_body.clone();

    let content_html = render_response_sse(&sse_req);

    let total_count = json_array_len(req.ws_first_response_events_json.as_deref());
    let total_html = total_count
        .map(|n| format!("<p>Total: {}</p>", n))
        .unwrap_or_default();

    let websearch_base = format!(
        "/_dashboard/sessions/{}/requests/{}/websearch",
        req.session_id, req.id
    );

    let content = view! {
        <h2>"Intercepted Response SSE"</h2>
        <div inner_html={total_html}/>
        <div inner_html={content_html}/>
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebSearch Intercepted Response SSE",
            session.name, req.id
        ),
        breadcrumbs: websearch_breadcrumbs(
            session,
            req,
            &[
                ("WebSearch", Some(websearch_base)),
                ("Intercepted Response", None),
            ],
        ),
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}

/// WebSearch Follow-up Hub — lists Messages/System/Tools/Params/Full JSON subpages.
pub fn render_websearch_followup_hub(req: &ProxyRequest, session: &Session) -> String {
    let followup_base = format!(
        "/_dashboard/sessions/{}/requests/{}/websearch/followup",
        req.session_id, req.id
    );
    let websearch_base = format!(
        "/_dashboard/sessions/{}/requests/{}/websearch",
        req.session_id, req.id
    );

    let mut subpages = Vec::new();

    if let Some(ref json_str) = req.ws_followup_body_json {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(obj) = val.as_object() {
                if let Some(messages) = obj.get("messages").and_then(|v| v.as_array()) {
                    subpages.push(Subpage::new(
                        "Messages",
                        format!("{}/messages", followup_base),
                        messages.len(),
                    ));
                }

                if let Some(system) = obj.get("system") {
                    let count = system
                        .as_array()
                        .map(|a| a.len().to_string())
                        .unwrap_or_default();
                    subpages.push(Subpage::new(
                        "System",
                        format!("{}/system", followup_base),
                        count,
                    ));
                }

                if let Some(tools) = obj.get("tools").and_then(|v| v.as_array()) {
                    subpages.push(Subpage::new(
                        "Tools",
                        format!("{}/tools", followup_base),
                        tools.len(),
                    ));
                }

                let skip_keys = ["messages", "system", "tools"];
                let params_count = obj
                    .keys()
                    .filter(|k| !skip_keys.contains(&k.as_str()))
                    .count();
                if params_count > 0 {
                    subpages.push(Subpage::new(
                        "Params",
                        format!("{}/params", followup_base),
                        params_count,
                    ));
                }

                subpages.push(Subpage::new(
                    "Full JSON",
                    format!("{}/full_json", followup_base),
                    String::new(),
                ));
            }
        }
    }

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebSearch Follow-up",
            session.name, req.id
        ),
        breadcrumbs: websearch_breadcrumbs(
            session,
            req,
            &[("WebSearch", Some(websearch_base)), ("Follow-up", None)],
        ),
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content: (),
        subpages,
    }
    .render()
}

/// WebSearch Follow-up subpage — renders Messages/System/Tools/Params/Full JSON using the same
/// renderers as the main request detail pages.
pub fn render_websearch_followup_page(
    req: &ProxyRequest,
    session: &Session,
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
        "full_json" => "Full JSON",
        _ => "Unknown",
    };

    let websearch_base = format!(
        "/_dashboard/sessions/{}/requests/{}/websearch",
        req.session_id, req.id
    );
    let followup_base = format!("{}/followup", websearch_base);

    let order = query
        .get("order")
        .cloned()
        .unwrap_or_else(|| "asc".to_string());

    let json_str = req.ws_followup_body_json.as_deref().unwrap_or("{}");
    let val: serde_json::Value =
        serde_json::from_str(json_str).unwrap_or(serde_json::Value::Object(Default::default()));
    let obj = val.as_object();

    let mut controls_html = String::new();

    let content_html = match page {
        "messages" => {
            if let Some(messages) = obj.and_then(|o| o.get("messages")) {
                let messages_json = serde_json::to_string(messages).unwrap_or_default();
                let toggle_order = if order == "desc" { "asc" } else { "desc" };
                let toggle_href = format!("{}/messages?order={}", followup_base, toggle_order);
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
                render_messages(&messages_json, &order, keep_tool_pairs)
            } else {
                "<p>No messages.</p>".to_string()
            }
        }
        "system" => {
            if let Some(system) = obj.and_then(|o| o.get("system")) {
                let system_json = serde_json::to_string(system).unwrap_or_default();
                render_system(&system_json, filters)
            } else {
                "<p>No system prompt.</p>".to_string()
            }
        }
        "tools" => {
            if let Some(tools) = obj.and_then(|o| o.get("tools")) {
                let tools_json = serde_json::to_string(tools).unwrap_or_default();
                render_tools(&tools_json, filters)
            } else {
                "<p>No tools.</p>".to_string()
            }
        }
        "params" => {
            if let Some(o) = obj {
                let skip_keys = ["messages", "system", "tools"];
                let params: serde_json::Map<String, serde_json::Value> = o
                    .iter()
                    .filter(|(k, _)| !skip_keys.contains(&k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                if params.is_empty() {
                    "<p>No params.</p>".to_string()
                } else {
                    let params_json = serde_json::to_string(&serde_json::Value::Object(params))
                        .unwrap_or_default();
                    render_kv_table(&params_json)
                }
            } else {
                "<p>No params.</p>".to_string()
            }
        }
        "full_json" => {
            let pretty =
                serde_json::to_string_pretty(&val).unwrap_or_else(|_| json_str.to_string());
            format!(
                r#"<textarea readonly rows="30" cols="80" wrap="off">{}</textarea>"#,
                html_escape(&pretty)
            )
        }
        _ => "<p>Unknown tab</p>".to_string(),
    };

    let total_count = match page {
        "messages" => obj
            .and_then(|o| o.get("messages"))
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        "system" => obj
            .and_then(|o| o.get("system"))
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        "tools" => obj
            .and_then(|o| o.get("tools"))
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        "params" => {
            let skip_keys = ["messages", "system", "tools"];
            obj.map(|o| {
                o.keys()
                    .filter(|k| !skip_keys.contains(&k.as_str()))
                    .count()
            })
        }
        _ => None,
    };
    let total_html = total_count
        .map(|n| format!("<p>Total: {}</p>", n))
        .unwrap_or_default();

    let content = view! {
        <h2>{page_label}</h2>
        <div inner_html={total_html}/>
        <div inner_html={controls_html}/>
        <div inner_html={content_html}/>
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebSearch Follow-up - {}",
            session.name, req.id, page_label
        ),
        breadcrumbs: websearch_breadcrumbs(
            session,
            req,
            &[
                ("WebSearch", Some(websearch_base)),
                ("Follow-up", Some(followup_base)),
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

/// Parse ws_rounds_json into a Vec of round objects.
fn parse_rounds(json: Option<&str>) -> Option<Vec<serde_json::Value>> {
    json.and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
}

/// WebSearch Round detail page — shows the full flow for a given round:
/// - Round 1: Intercepted Response SSE + Follow-up Request + Response SSE
/// - Round 2+: Follow-up Request + Response SSE
pub fn render_websearch_round(req: &ProxyRequest, session: &Session, round_idx: usize) -> String {
    let websearch_base = format!(
        "/_dashboard/sessions/{}/requests/{}/websearch",
        req.session_id, req.id
    );

    let rounds = parse_rounds(req.ws_rounds_json.as_deref()).unwrap_or_default();
    let round = rounds.get(round_idx);

    let round_label = format!("Round {}", round_idx + 1);

    // Extract decision and tool names from round data
    let decision = round
        .and_then(|r| r.get("decision"))
        .and_then(|v| v.as_str())
        .unwrap_or("--");
    let tool_names: Vec<&str> = round
        .and_then(|r| r.get("tool_names"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let tools_display = if tool_names.is_empty() {
        "--".to_string()
    } else {
        tool_names.join(", ")
    };

    // For round 1, show the intercepted response SSE
    let intercepted_html = if round_idx == 0 {
        if req.ws_first_response_events_json.is_some() {
            let mut sse_req = req.clone();
            sse_req.response_events_json = req.ws_first_response_events_json.clone();
            sse_req.response_body = req.ws_first_response_body.clone();
            let sse_html = render_response_sse(&sse_req);
            format!("<h2>Intercepted Response</h2>{}", sse_html)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let followup_html = if let Some(request_id) = round
        .and_then(|r| r.get("request_id"))
        .and_then(|v| v.as_str())
    {
        let href = format!(
            "/_dashboard/sessions/{}/requests/{}",
            req.session_id, request_id
        );
        format!(
            r#"<h2>Follow-up Request</h2><p><a href="{}">View request #{}</a></p>"#,
            html_escape(&href),
            html_escape(request_id),
        )
    } else if let Some(body) = round.and_then(|r| r.get("followup_body")) {
        let pretty = serde_json::to_string_pretty(body).unwrap_or_else(|_| body.to_string());
        format!(
            r#"<h2>Follow-up Request</h2><textarea readonly rows="20" cols="80" wrap="off">{}</textarea>"#,
            html_escape(&pretty)
        )
    } else {
        "<h2>Follow-up Request</h2><p>No data.</p>".to_string()
    };

    let response_html = if let Some(events) = round.and_then(|r| r.get("response_events")) {
        let events_json = serde_json::to_string(events).unwrap_or_default();
        let mut sse_req = req.clone();
        sse_req.response_events_json = Some(events_json);
        sse_req.response_body = round
            .and_then(|r| r.get("response_body"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let sse_html = render_response_sse(&sse_req);
        format!("<h2>Response</h2>{}", sse_html)
    } else {
        "<h2>Response</h2><p>No data.</p>".to_string()
    };

    let content = view! {
        <div inner_html={intercepted_html} />
        <div inner_html={followup_html} />
        <div inner_html={response_html} />
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Request #{} - WebSearch {}",
            session.name, req.id, round_label
        ),
        breadcrumbs: websearch_breadcrumbs(
            session,
            req,
            &[("WebSearch", Some(websearch_base)), (&round_label, None)],
        ),
        nav_links: vec![NavLink::back()],
        info_rows: vec![
            InfoRow::new("Decision", decision),
            InfoRow::new("Tools", &tools_display),
        ],
        content,
        subpages: vec![],
    }
    .render()
}
