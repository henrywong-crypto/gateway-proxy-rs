use crate::pages::{collapsible_block, html_escape};

pub fn render_messages(json_str: &str, order: &str) -> String {
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
