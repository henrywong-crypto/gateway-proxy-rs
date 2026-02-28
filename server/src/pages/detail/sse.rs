use crate::pages::{collapsible_block, html_escape};
use ::common::models::ProxyRequest;

pub fn render_response_sse(req: &ProxyRequest) -> String {
    let mut html = String::new();

    // SSE events
    if let Some(ref events_json) = req.response_events_json {
        if let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(events_json) {
            html.push_str(&format!("{} SSE events", events.len()));
            html.push_str("<table><tr><th>#</th><th>Event</th><th>Data</th><th>Raw</th></tr>");

            // Track accumulated text/json per content block index
            let mut block_text: std::collections::HashMap<i64, String> =
                std::collections::HashMap::new();
            let mut block_json: std::collections::HashMap<i64, String> =
                std::collections::HashMap::new();
            let mut block_names: std::collections::HashMap<i64, String> =
                std::collections::HashMap::new();
            let mut block_types: std::collections::HashMap<i64, String> =
                std::collections::HashMap::new();

            for (i, event) in events.iter().enumerate() {
                let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
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
                                let text =
                                    delta.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                                block_text.entry(index).or_default().push_str(text);
                            }
                            "input_json_delta" => {
                                let json = delta
                                    .get("partial_json")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
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
                            .and_then(|v| serde_json::to_string_pretty(&v))
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

pub fn summarize_sse_event(event_type: &str, data: &serde_json::Value) -> String {
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
            let mut parts = vec![format!(
                "{} {} {}",
                html_escape(model),
                html_escape(role),
                html_escape(id)
            )];
            for key in &[
                "input_tokens",
                "output_tokens",
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
            ] {
                if let Some(tokens) = data
                    .pointer(&format!("/message/usage/{}", key))
                    .and_then(|v| v.as_i64())
                {
                    parts.push(format!("{}: {}", key, tokens));
                }
            }
            parts.join(" | ")
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
            let mut parts = Vec::new();
            if !stop_reason.is_empty() {
                parts.push(format!("stop: {}", html_escape(stop_reason)));
            }
            for key in &[
                "input_tokens",
                "output_tokens",
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
            ] {
                if let Some(tokens) = data
                    .pointer(&format!("/usage/{}", key))
                    .and_then(|v| v.as_i64())
                {
                    parts.push(format!("{}: {}", key, tokens));
                }
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
