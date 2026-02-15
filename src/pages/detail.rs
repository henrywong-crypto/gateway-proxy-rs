use leptos::prelude::*;

use crate::models::{ProxyRequest, Session};
use crate::pages::{collapsible_block, html_escape, page_layout};

fn breadcrumb_html(session: &Session, req: &ProxyRequest, current_page: Option<(&str, &str)>) -> String {
    let session_href = format!("/_dashboard/sessions/{}", req.session_id);
    let requests_href = format!("/_dashboard/sessions/{}/requests", req.session_id);
    let req_href = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, req.id);
    if let Some((_key, label)) = current_page {
        let page_part = format!(r#" / {}"#, html_escape(label));
        let view = view! {
            <h1>
                <a href="/_dashboard">"Home"</a>
                " / "
                <a href="/_dashboard/sessions">"Sessions"</a>
                " / "
                <a href={session_href}>{format!("Session {}", session.name)}</a>
                " / "
                <a href={requests_href}>"Requests"</a>
                " / "
                <a href={req_href}>{format!("Request #{}", req.id)}</a>
                <span inner_html={page_part}/>
            </h1>
        };
        view.to_html()
    } else {
        let view = view! {
            <h1>
                <a href="/_dashboard">"Home"</a>
                " / "
                <a href="/_dashboard/sessions">"Sessions"</a>
                " / "
                <a href={session_href}>{format!("Session {}", session.name)}</a>
                " / "
                <a href={requests_href}>"Requests"</a>
                " / "
                {format!("Request #{}", req.id)}
            </h1>
        };
        view.to_html()
    }
}

pub fn render_detail_overview(
    req: &ProxyRequest,
    session: &Session,
) -> String {
    let req = req.clone();
    let title = format!("Gateway Proxy - Session {} - Request #{}", session.name, req.id);

    let method = html_escape(&req.method);
    let path = html_escape(&req.path);
    let model = req.model.as_deref().map(|m| html_escape(m)).unwrap_or_default();
    let timestamp = html_escape(&req.timestamp);

    let base = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, req.id);
    let has_response = req.response_body.is_some() || req.response_events_json.is_some();

    let make_table = |items: Vec<(&str, &str, bool)>| -> String {
        let rows: String = items.into_iter()
            .filter(|(_, _, available)| *available)
            .map(|(key, label, _)| {
                format!(r#"<tr><td><a href="{}/{}">{}</a></td></tr>"#, base, key, html_escape(label))
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

    let response_links = make_table(vec![
        ("response_sse", "SSE", req.response_events_json.is_some()),
    ]);

    let requests_href = format!("/_dashboard/sessions/{}/requests", req.session_id);

    let bc = breadcrumb_html(session, &req, None);

    let body = view! {
        <div inner_html={bc}/>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href={requests_href}>"Back"</a></td></tr>
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
    let title = format!("Gateway Proxy - Session {} - Request #{} - {}", session.name, req.id, page_label);

    let truncate = query.get("truncate").map(|v| v.as_str()) != Some("off");
    let order = query.get("order").cloned().unwrap_or_else(|| "desc".to_string());

    let base = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, req.id);

    let mut controls_html = String::new();

    let content_html = match page {
        "messages" => {
            if let Some(ref json_str) = req.messages_json {
                let toggle_order = if order == "desc" { "asc" } else { "desc" };
                let toggle_href = format!("{}/messages?order={}", base, toggle_order);
                controls_html = format!(
                    r#"<div>Showing: {} | <a href="{}">{}</a></div>"#,
                    if order == "desc" { "newest first" } else { "oldest first" },
                    toggle_href,
                    format!("Switch to {}", if order == "desc" { "oldest first" } else { "newest first" })
                );
                render_messages(json_str, &order)
            } else {
                "<p>No messages.</p>".to_string()
            }
        }
        "system" => req
            .system_json
            .as_deref()
            .map(render_system)
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
            controls_html = format!(
                r#"<a href="{}">{}</a>"#,
                toggle_href,
                toggle_label,
            );
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
    let back_href = format!("/_dashboard/sessions/{}/requests/{}", req.session_id, req.id);

    let body = view! {
        <div inner_html={bc}/>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href={back_href}>"Back"</a></td></tr>
        </table>
        <h2>{page_label}</h2>
        <div inner_html={controls_html}/>
        <div inner_html={content_html}/>
    };

    page_layout(&title, body.to_html())
}

fn render_messages(json_str: &str, order: &str) -> String {
    let Ok(mut msgs) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    if order == "desc" {
        msgs.reverse();
    }

    let mut html = String::from("<table><tr><th>Role</th><th>Type</th><th>Content</th></tr>");
    for msg in &msgs {
        let role = msg
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown");

        let content = &msg["content"];
        if let Some(s) = content.as_str() {
            html.push_str(&format!(
                "<tr><td>{}</td><td>text</td><td>{}</td></tr>",
                html_escape(role),
                collapsible_block(s, "")
            ));
        } else if let Some(blocks) = content.as_array() {
            for (i, block) in blocks.iter().enumerate() {
                let btype = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let role_cell = if i == 0 {
                    html_escape(role)
                } else {
                    String::new()
                };
                match btype {
                    "text" => {
                        let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>text</td><td>{}</td></tr>",
                            role_cell,
                            collapsible_block(text, "")
                        ));
                    }
                    "thinking" => {
                        let text = block
                            .get("thinking")
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>thinking</td><td>{}</td></tr>",
                            role_cell,
                            collapsible_block(text, "")
                        ));
                    }
                    "tool_use" => {
                        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let mut params_html = String::new();
                        if let Some(input) = block.get("input").and_then(|i| i.as_object()) {
                            params_html.push_str("<table><tr><th>Param</th><th>Value</th></tr>");
                            for (k, v) in input {
                                let val = if v.is_string() {
                                    v.as_str().unwrap_or("").to_string()
                                } else {
                                    serde_json::to_string(v).unwrap_or_default()
                                };
                                params_html.push_str(&format!(
                                    "<tr><td>{}</td><td>{}</td></tr>",
                                    html_escape(k),
                                    collapsible_block(&val, "")
                                ));
                            }
                            params_html.push_str("</table>");
                        }
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>tool_use: {} {}</td><td>{}</td></tr>",
                            role_cell,
                            html_escape(name),
                            html_escape(id),
                            params_html
                        ));
                    }
                    "tool_result" => {
                        let tool_use_id = block
                            .get("tool_use_id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("");
                        let result_text = if let Some(s) =
                            block.get("content").and_then(|c| c.as_str())
                        {
                            s.to_string()
                        } else if let Some(arr) =
                            block.get("content").and_then(|c| c.as_array())
                        {
                            arr.iter()
                                .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            String::new()
                        };
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>tool_result {}</td><td>{}</td></tr>",
                            role_cell,
                            html_escape(tool_use_id),
                            collapsible_block(&result_text, "")
                        ));
                    }
                    _ => {}
                }
            }
        } else {
            html.push_str(&format!(
                "<tr><td>{}</td><td></td><td></td></tr>",
                html_escape(role)
            ));
        }
    }
    html.push_str("</table>");
    html
}

fn render_system(json_str: &str) -> String {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    if let Some(s) = val.as_str() {
        return format!(
            "<table><tr><th>Type</th><th>Content</th></tr><tr><td>text</td><td>{}</td></tr></table>",
            collapsible_block(s, "")
        );
    }

    if let Some(arr) = val.as_array() {
        let mut html = String::from("<table><tr><th>Type</th><th>Content</th></tr>");
        for block in arr {
            let btype = block.get("type").and_then(|t| t.as_str()).unwrap_or("text");
            let text = block
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let fallback;
            let text = if text.is_empty() {
                fallback = serde_json::to_string_pretty(block).unwrap_or_default();
                &fallback
            } else {
                text
            };
            let cache_info = block
                .get("cache_control")
                .and_then(|c| c.get("type"))
                .and_then(|t| t.as_str())
                .map(|t| format!(" (cache: {})", html_escape(t)))
                .unwrap_or_default();
            html.push_str(&format!(
                "<tr><td>{}{}</td><td>{}</td></tr>",
                html_escape(btype),
                cache_info,
                collapsible_block(text, "")
            ));
        }
        html.push_str("</table>");
        return html;
    }

    format!(
        "<pre>{}</pre>",
        html_escape(&serde_json::to_string_pretty(&val).unwrap_or_default())
    )
}

fn render_tools(json_str: &str) -> String {
    let Ok(tools) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    let mut html = String::from("<table><tr><th>Name</th><th>Description</th><th>Parameters</th></tr>");
    for tool in &tools {
        let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("(unnamed)");
        let desc = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        let mut params_html = String::new();
        if let Some(schema) = tool.get("input_schema").and_then(|s| s.as_object()) {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                let required: Vec<&str> = schema
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect()
                    })
                    .unwrap_or_default();

                params_html.push_str("<table><tr><th>Name</th><th>Type</th><th>Req</th><th>Description</th></tr>");
                for (k, v) in props {
                    let ptype = v
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("object");
                    let pdesc = v
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    let req = if required.contains(&k.as_str()) {
                        "yes"
                    } else {
                        "no"
                    };
                    params_html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(k),
                        html_escape(ptype),
                        req,
                        html_escape(pdesc)
                    ));
                }
                params_html.push_str("</table>");
            }
        }

        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(name),
            collapsible_block(desc, ""),
            params_html
        ));
    }
    html.push_str("</table>");
    html
}

fn render_kv_table(json_str: &str) -> String {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    let Some(obj) = val.as_object() else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    let mut html = String::from(
        "<table><tr><th>Key</th><th>Value</th></tr>",
    );
    for (k, v) in obj {
        let val_str = if v.is_string() {
            v.as_str().unwrap_or("").to_string()
        } else {
            serde_json::to_string_pretty(v).unwrap_or_default()
        };
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td></tr>",
            html_escape(k),
            collapsible_block(&val_str, "")
        ));
    }
    html.push_str("</table>");
    html
}

fn render_response_headers(req: &ProxyRequest) -> String {
    let mut html = String::new();

    if let Some(status) = req.response_status {
        html.push_str(&format!("<div><strong>Status:</strong> {}</div>", status));
    }

    if let Some(ref headers) = req.response_headers_json {
        html.push_str(&render_kv_table(headers));
    }

    if html.is_empty() {
        html.push_str("<p>No response headers.</p>");
    }

    html
}

fn render_response_sse(req: &ProxyRequest) -> String {
    let mut html = String::new();

    // SSE events
    if let Some(ref events_json) = req.response_events_json {
        if let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(events_json) {
            html.push_str(&format!("{} SSE events", events.len()));
            html.push_str("<table><tr><th>#</th><th>Event</th><th>Data</th><th>Raw</th></tr>");

            // Track accumulated text/json per content block index
            let mut block_text: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
            let mut block_json: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
            let mut block_names: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
            let mut block_types: std::collections::HashMap<i64, String> = std::collections::HashMap::new();

            for (i, event) in events.iter().enumerate() {
                let event_type = event
                    .get("event")
                    .and_then(|e| e.as_str())
                    .unwrap_or("");
                let data = &event["data"];

                // Accumulate state
                match event_type {
                    "content_block_start" => {
                        let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
                        let btype = data
                            .pointer("/content_block/type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let name = data
                            .pointer("/content_block/name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        block_types.insert(index, btype);
                        if !name.is_empty() {
                            block_names.insert(index, name);
                        }
                        block_text.remove(&index);
                        block_json.remove(&index);
                    }
                    "content_block_delta" => {
                        let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
                        let delta = &data["delta"];
                        let dtype = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        match dtype {
                            "text_delta" => {
                                let text = delta.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                block_text.entry(index).or_default().push_str(text);
                            }
                            "thinking_delta" => {
                                let text = delta.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                                block_text.entry(index).or_default().push_str(text);
                            }
                            "input_json_delta" => {
                                let json = delta.get("partial_json").and_then(|v| v.as_str()).unwrap_or("");
                                block_json.entry(index).or_default().push_str(json);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }

                let summary = summarize_sse_event(event_type, data);
                let raw = serde_json::to_string_pretty(data).unwrap_or_default();
                let raw_html = format!(
                    r#"<details class="collapsible"><summary><span class="show-more">show raw</span></summary><pre class="collapsible-full">{}</pre></details>"#,
                    html_escape(&raw),
                );
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    i + 1,
                    html_escape(event_type),
                    summary,
                    raw_html,
                ));

                // Insert summary row after content_block_stop
                if event_type == "content_block_stop" {
                    let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
                    let btype = block_types.get(&index).map(|s| s.as_str()).unwrap_or("");
                    let name = block_names.get(&index).map(|s| s.as_str()).unwrap_or("");

                    let label = if !name.is_empty() {
                        format!("{} — {}", html_escape(btype), html_escape(name))
                    } else {
                        html_escape(btype)
                    };

                    let content = if let Some(json_str) = block_json.get(&index) {
                        let formatted = serde_json::from_str::<serde_json::Value>(json_str)
                            .and_then(|v| serde_json::to_string_pretty(&v).map_err(|e| e.into()))
                            .unwrap_or_else(|_| json_str.clone());
                        collapsible_block(&formatted, "")
                    } else if let Some(text) = block_text.get(&index) {
                        collapsible_block(text, "")
                    } else {
                        String::new()
                    };

                    if !content.is_empty() {
                        html.push_str(&format!(
                            "<tr><td></td><td><strong>{}</strong></td><td colspan=\"2\">{}</td></tr>",
                            label,
                            content,
                        ));
                    }
                }
            }
            html.push_str("</table>");
        }
    } else if let Some(ref body) = req.response_body {
        html.push_str(&format!("<pre>{}</pre>", html_escape(body)));
    }

    html
}

fn summarize_sse_event(event_type: &str, data: &serde_json::Value) -> String {
    match event_type {
        "message_start" => {
            let model = data
                .pointer("/message/model")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let role = data
                .pointer("/message/role")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let id = data
                .pointer("/message/id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("{} {} {}", html_escape(model), html_escape(role), html_escape(id))
        }
        "content_block_start" => {
            let btype = data
                .pointer("/content_block/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            let name = data
                .pointer("/content_block/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name.is_empty() {
                format!("[{}] {}", index, html_escape(btype))
            } else {
                format!("[{}] {} {}", index, html_escape(btype), html_escape(name))
            }
        }
        "content_block_delta" => {
            let delta = &data["delta"];
            let dtype = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match dtype {
                "text_delta" => {
                    let text = delta.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let preview = if text.len() > 80 {
                        format!("{}...", &text[..80])
                    } else {
                        text.to_string()
                    };
                    html_escape(&preview)
                }
                "thinking_delta" => {
                    let text = delta.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                    let preview = if text.len() > 80 {
                        format!("{}...", &text[..80])
                    } else {
                        text.to_string()
                    };
                    html_escape(&preview)
                }
                "input_json_delta" => {
                    let json = delta
                        .get("partial_json")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let preview = if json.len() > 80 {
                        format!("{}...", &json[..80])
                    } else {
                        json.to_string()
                    };
                    html_escape(&preview)
                }
                _ => {
                    let s = serde_json::to_string(delta).unwrap_or_default();
                    html_escape(&s)
                }
            }
        }
        "content_block_stop" => {
            let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("[{}]", index)
        }
        "message_delta" => {
            let stop_reason = data
                .pointer("/delta/stop_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let output_tokens = data
                .pointer("/usage/output_tokens")
                .and_then(|v| v.as_i64());
            let mut parts = Vec::new();
            if !stop_reason.is_empty() {
                parts.push(format!("stop: {}", html_escape(stop_reason)));
            }
            if let Some(tokens) = output_tokens {
                parts.push(format!("output_tokens: {}", tokens));
            }
            parts.join(" | ")
        }
        "message_stop" => String::new(),
        _ => {
            let s = serde_json::to_string(data).unwrap_or_default();
            let preview = if s.len() > 120 {
                format!("{}...", &s[..120])
            } else {
                s
            };
            html_escape(&preview)
        }
    }
}
