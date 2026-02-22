use crate::pages::{collapsible_block, html_escape};

pub fn render_messages(json_str: &str, order: &str, keep_tool_pairs: i64) -> String {
    let Ok(mut msgs) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    // Collect tool_use IDs to determine which are filtered
    let filtered_ids: std::collections::HashSet<String> = if keep_tool_pairs > 0 {
        let mut all_ids: Vec<String> = Vec::new();
        for msg in &msgs {
            if let Some(blocks) = msg.get("content").and_then(|c| c.as_array()) {
                for block in blocks {
                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        if let Some(id) = block.get("id").and_then(|i| i.as_str()) {
                            all_ids.push(id.to_string());
                        }
                    }
                }
            }
        }
        let keep = keep_tool_pairs as usize;
        if all_ids.len() > keep {
            all_ids[..all_ids.len() - keep].iter().cloned().collect()
        } else {
            std::collections::HashSet::new()
        }
    } else {
        std::collections::HashSet::new()
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

                // Determine if this block is filtered
                let is_filtered = match btype {
                    "tool_use" => {
                        let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        filtered_ids.contains(id)
                    }
                    "tool_result" => {
                        let id = block
                            .get("tool_use_id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("");
                        filtered_ids.contains(id)
                    }
                    _ => false,
                };
                let row_class = if is_filtered {
                    " class=\"filtered-row\""
                } else {
                    ""
                };
                let filtered_badge = if is_filtered {
                    " <span class=\"filtered-badge\">[FILTERED]</span>"
                } else {
                    ""
                };

                match btype {
                    "text" => {
                        let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        let cache_info = cache_control_label(block);
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>text{}</td><td>{}</td></tr>",
                            role_cell,
                            cache_info,
                            collapsible_block(text, "")
                        ));
                    }
                    "thinking" => {
                        let text = block.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
                        let cache_info = cache_control_label(block);
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>thinking{}</td><td>{}</td></tr>",
                            role_cell,
                            cache_info,
                            collapsible_block(text, "")
                        ));
                    }
                    "tool_use" => {
                        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let cache_info = cache_control_label(block);
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
                            "<tr{}><td>{}</td><td>tool_use{}{}: {} {}</td><td>{}</td></tr>",
                            row_class,
                            role_cell,
                            cache_info,
                            filtered_badge,
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
                        let cache_info = cache_control_label(block);
                        let result_text = if let Some(s) =
                            block.get("content").and_then(|c| c.as_str())
                        {
                            s.to_string()
                        } else if let Some(arr) = block.get("content").and_then(|c| c.as_array()) {
                            arr.iter()
                                .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            String::new()
                        };
                        html.push_str(&format!(
                            "<tr{}><td>{}</td><td>tool_result{}{} {}</td><td>{}</td></tr>",
                            row_class,
                            role_cell,
                            cache_info,
                            filtered_badge,
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

fn cache_control_label(block: &serde_json::Value) -> String {
    block
        .get("cache_control")
        .and_then(|c| c.get("type"))
        .and_then(|t| t.as_str())
        .map(|t| format!(" (cache: {})", html_escape(t)))
        .unwrap_or_default()
}
