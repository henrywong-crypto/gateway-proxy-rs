use crate::pages::{collapsible_block, html_escape};

pub fn render_system(json_str: &str) -> String {
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
