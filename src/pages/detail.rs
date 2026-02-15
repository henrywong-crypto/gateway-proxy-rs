use leptos::prelude::*;

use crate::models::ProxyRequest;
use crate::pages::{collapsible_block, html_escape, page_layout};

pub fn render_detail(
    req: &ProxyRequest,
    active_tab: &str,
    query: &std::collections::HashMap<String, String>,
) -> String {
    let req = req.clone();
    let active_tab = active_tab.to_string();
    let title = format!("Request #{} - {}", req.id, req.method);

    let truncate = query.get("truncate").map(|v| v.as_str()) != Some("off");
    let order = query.get("order").cloned().unwrap_or_else(|| "desc".to_string());

    let back_href = format!("/__proxy__/s/{}", req.session_id);
    let model_html = req
        .model
        .as_deref()
        .map(|m| format!(r#" <span class="model">{}</span>"#, html_escape(m)))
        .unwrap_or_default();
    let header_html = format!(
        r#"<span class="method">{}</span> <span class="path">{}</span>{} <span class="time">{}</span>"#,
        html_escape(&req.method),
        html_escape(&req.path),
        model_html,
        html_escape(&req.timestamp),
    );

    let base = format!("/__proxy__/s/{}/r/{}", req.session_id, req.id);
    let has_response = req.response_body.is_some() || req.response_events_json.is_some();
    let tabs: Vec<(&str, &str, bool)> = vec![
        ("messages", "Messages", req.messages_json.is_some()),
        ("system", "System", req.system_json.is_some()),
        ("tools", "Tools", req.tools_json.is_some()),
        ("params", "Params", req.params_json.is_some()),
        ("headers", "Headers", true),
        ("full_json", "Full JSON", true),
        ("response", "Response", has_response),
    ];

    // Controls above tab content
    let mut controls_html = String::new();

    let tab_content_html = match active_tab.as_str() {
        "messages" => {
            if let Some(ref json_str) = req.messages_json {
                let toggle_order = if order == "desc" { "asc" } else { "desc" };
                let toggle_label = if order == "desc" {
                    "Newest first"
                } else {
                    "Oldest first"
                };
                let toggle_href = format!("{}?tab=messages&order={}", base, toggle_order);
                controls_html = format!(
                    r#"<div style="margin-bottom:8px;font-size:12px;color:#888">Showing: {} | <a href="{}">{}</a></div>"#,
                    if order == "desc" { "newest first" } else { "oldest first" },
                    toggle_href,
                    format!("Switch to {}", if order == "desc" { "oldest first" } else { "newest first" })
                );
                render_messages(json_str, &order)
            } else {
                String::new()
            }
        }
        "system" => req
            .system_json
            .as_deref()
            .map(render_system)
            .unwrap_or_default(),
        "tools" => req
            .tools_json
            .as_deref()
            .map(render_tools)
            .unwrap_or_default(),
        "params" => req
            .params_json
            .as_deref()
            .map(render_kv_table)
            .unwrap_or_default(),
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
                "{}?tab=full_json&truncate={}",
                base,
                if truncate { "off" } else { "on" }
            );
            let toggle_label = if truncate {
                "Show full strings"
            } else {
                "Show truncated"
            };
            let full_json_for_copy = html_escape(
                req.body_json.as_deref().unwrap_or(""),
            );
            controls_html = format!(
                r#"<div style="margin-bottom:8px;font-size:12px"><a href="{}">{}</a><span style="margin-left:12px"><button class="copy-btn" onclick="navigator.clipboard.writeText(document.getElementById('full-json-data').textContent).then(function(){{var b=event.target;b.textContent='Copied!';setTimeout(function(){{b.textContent='Copy Full JSON'}},1500)}})">Copy Full JSON</button></span></div><textarea id="full-json-data" style="display:none">{}</textarea>"#,
                toggle_href,
                toggle_label,
                full_json_for_copy
            );
            format!("<pre>{}</pre>", html_escape(json))
        }
        "response" => render_response(&req),
        _ => "<p>Unknown tab</p>".to_string(),
    };

    let body = view! {
        <p style="margin-bottom:12px">
            <a href={back_href}>"\u{2190} Back to session"</a>
        </p>
        <div style="margin-bottom:12px;padding:8px 0;border-bottom:1px solid #333" inner_html={header_html}/>
        <div class="tab-bar">
            {tabs.into_iter().filter(|(_, _, available)| *available).map(|(key, label, _)| {
                let class = if key == active_tab { "tab active" } else { "tab" };
                let href = format!("{}?tab={}", base, key);
                view! {
                    <a class={class} href={href}>{label}</a>
                }
            }).collect::<Vec<_>>()}
        </div>
        <div inner_html={controls_html}/>
        <div inner_html={tab_content_html}/>
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

    let mut html = String::new();
    for msg in &msgs {
        let role = msg
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown");
        let role_class = match role {
            "user" => "role-user",
            "assistant" => "role-assistant",
            _ => "",
        };

        html.push_str(&format!(
            r#"<div class="msg-card {}"><div class="msg-role {}">{}</div>"#,
            role_class, role, role
        ));

        let content = &msg["content"];
        if let Some(s) = content.as_str() {
            html.push_str(&format!(
                r#"<div class="msg-block">{}</div>"#,
                collapsible_block(s, "msg-block-text")
            ));
        } else if let Some(blocks) = content.as_array() {
            for block in blocks {
                let btype = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match btype {
                    "text" => {
                        let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        html.push_str(&format!(
                            r#"<div class="msg-block"><div class="msg-block-label">text</div>{}</div>"#,
                            collapsible_block(text, "msg-block-text")
                        ));
                    }
                    "thinking" => {
                        let text = block
                            .get("thinking")
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        html.push_str(&format!(
                            r#"<div class="msg-block"><div class="msg-block-label">thinking</div>{}</div>"#,
                            collapsible_block(text, "msg-block-thinking")
                        ));
                    }
                    "tool_use" => {
                        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        html.push_str(&format!(
                            r#"<div class="msg-block"><div class="msg-block-label">tool_use</div><div class="msg-block-tool"><span class="tool-name">{}</span> <span style="color:#888;font-size:11px">{}</span></div>"#,
                            html_escape(name),
                            html_escape(id)
                        ));
                        if let Some(input) = block.get("input").and_then(|i| i.as_object()) {
                            html.push_str(r#"<table style="width:100%;border-collapse:collapse;font-size:12px;margin-top:6px"><tr><th style="text-align:left;color:#569cd6;padding:4px 8px;border-bottom:1px solid #444">Param</th><th style="text-align:left;color:#569cd6;padding:4px 8px;border-bottom:1px solid #444">Value</th></tr>"#);
                            for (k, v) in input {
                                let val = if v.is_string() {
                                    v.as_str().unwrap_or("").to_string()
                                } else {
                                    serde_json::to_string(v).unwrap_or_default()
                                };
                                html.push_str(&format!(
                                    r#"<tr><td style="color:#dcdcaa;padding:4px 8px;border-bottom:1px solid #333;white-space:nowrap">{}</td><td style="padding:4px 8px;border-bottom:1px solid #333">{}</td></tr>"#,
                                    html_escape(k),
                                    collapsible_block(&val, "")
                                ));
                            }
                            html.push_str("</table>");
                        }
                        html.push_str("</div>");
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
                            r#"<div class="msg-block"><div class="msg-block-label">tool_result <span style="color:#888;font-size:10px">{}</span></div>{}</div>"#,
                            html_escape(tool_use_id),
                            collapsible_block(&result_text, "msg-block-result")
                        ));
                    }
                    _ => {}
                }
            }
        }

        html.push_str("</div>");
    }
    html
}

fn render_system(json_str: &str) -> String {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    if let Some(s) = val.as_str() {
        return format!(
            r#"<div class="card">{}</div>"#,
            collapsible_block(s, "")
        );
    }

    if let Some(arr) = val.as_array() {
        let mut html = String::new();
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
                .map(|t| format!(r#" <span style="color:#4ec9b0">cache: {}</span>"#, html_escape(t)))
                .unwrap_or_default();
            html.push_str(&format!(
                r#"<div class="card"><div class="msg-block-label">{}{}</div>{}</div>"#,
                html_escape(btype),
                cache_info,
                collapsible_block(text, "")
            ));
        }
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

    let mut html = String::new();
    for tool in &tools {
        let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("(unnamed)");
        let desc = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        html.push_str(&format!(
            r#"<div class="card"><h3>{}</h3>"#,
            html_escape(name)
        ));

        if !desc.is_empty() {
            html.push_str(&collapsible_block(desc, "tool-desc"));
        }

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

                html.push_str(r#"<table><tr><th>Name</th><th>Type</th><th>Required</th><th>Description</th></tr>"#);
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
                    html.push_str(&format!(
                        r#"<tr><td style="color:#dcdcaa">{}</td><td style="color:#4ec9b0">{}</td><td style="color:#ce9178">{}</td><td>{}</td></tr>"#,
                        html_escape(k),
                        html_escape(ptype),
                        req,
                        html_escape(pdesc)
                    ));
                }
                html.push_str("</table>");
            }
        }

        html.push_str("</div>");
    }
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
        r#"<div class="card"><table class="kv-table"><tr><th>Key</th><th>Value</th></tr>"#,
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
    html.push_str("</table></div>");
    html
}

fn render_response(req: &ProxyRequest) -> String {
    let mut html = String::new();

    // Status code
    if let Some(status) = req.response_status {
        let status_class = if status >= 200 && status < 300 {
            "color:#4ec9b0"
        } else if status >= 400 {
            "color:#f44747"
        } else {
            "color:#dcdcaa"
        };
        html.push_str(&format!(
            r#"<div class="card"><strong>Status:</strong> <span style="{}">{}</span></div>"#,
            status_class, status
        ));
    }

    // Response headers
    if let Some(ref headers) = req.response_headers_json {
        html.push_str(&render_kv_table(headers));
    }

    // SSE events
    if let Some(ref events_json) = req.response_events_json {
        if let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(events_json) {
            html.push_str(&format!(
                r#"<div style="margin:12px 0 8px;font-size:12px;color:#888">{} SSE events</div>"#,
                events.len()
            ));
            html.push_str(r#"<div class="card"><table class="kv-table"><tr><th>#</th><th>Event</th><th>Data</th><th>Raw</th></tr>"#);

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
                let event_color = match event_type {
                    "message_start" | "message_stop" => "#569cd6",
                    "message_delta" => "#c586c0",
                    "content_block_start" | "content_block_stop" => "#4ec9b0",
                    "content_block_delta" => "#dcdcaa",
                    _ => "#d4d4d4",
                };
                html.push_str(&format!(
                    r#"<tr><td style="color:#888;white-space:nowrap">{}</td><td style="color:{};white-space:nowrap">{}</td><td>{}</td><td>{}</td></tr>"#,
                    i + 1,
                    event_color,
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
                        format!(
                            r#"<span style="color:#569cd6">{}</span> <span style="color:#dcdcaa">{}</span>"#,
                            html_escape(btype),
                            html_escape(name),
                        )
                    } else {
                        format!(
                            r#"<span style="color:#569cd6">{}</span>"#,
                            html_escape(btype),
                        )
                    };

                    let content = if let Some(json_str) = block_json.get(&index) {
                        // Tool use: show parsed JSON arguments
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
                            r#"<tr style="background:#1a1a2e"><td style="color:#888"></td><td style="white-space:nowrap">{}</td><td colspan="2">{}</td></tr>"#,
                            label,
                            content,
                        ));
                    }
                }
            }
            html.push_str("</table></div>");
        }
    } else if let Some(ref body) = req.response_body {
        // Non-SSE response body
        html.push_str(&format!(
            r#"<div class="card"><pre>{}</pre></div>"#,
            html_escape(body)
        ));
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
            format!(
                r#"<span style="color:#4ec9b0">{}</span> <span style="color:#ce9178">{}</span> <span style="color:#888;font-size:11px">{}</span>"#,
                html_escape(model),
                html_escape(role),
                html_escape(id),
            )
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
                format!(
                    r#"[{}] <span style="color:#569cd6">{}</span>"#,
                    index,
                    html_escape(btype),
                )
            } else {
                format!(
                    r#"[{}] <span style="color:#569cd6">{}</span> <span style="color:#dcdcaa">{}</span>"#,
                    index,
                    html_escape(btype),
                    html_escape(name),
                )
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
                    format!(
                        r#"<span style="color:#ce9178">{}</span>"#,
                        html_escape(&preview),
                    )
                }
                "thinking_delta" => {
                    let text = delta.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                    let preview = if text.len() > 80 {
                        format!("{}...", &text[..80])
                    } else {
                        text.to_string()
                    };
                    format!(
                        r#"<span style="color:#808080;font-style:italic">{}</span>"#,
                        html_escape(&preview),
                    )
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
                    format!(
                        r#"<span style="color:#d7ba7d">{}</span>"#,
                        html_escape(&preview),
                    )
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
                parts.push(format!(
                    r#"stop: <span style="color:#c586c0">{}</span>"#,
                    html_escape(stop_reason),
                ));
            }
            if let Some(tokens) = output_tokens {
                parts.push(format!(
                    r#"output_tokens: <span style="color:#b5cea8">{}</span>"#,
                    tokens,
                ));
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
