use regex::Regex;

use crate::pages::{collapsible_block, html_escape};

fn matched_filter<'a>(text: &str, filters: &'a [String]) -> Option<&'a str> {
    filters.iter().find_map(|f| {
        let matched = match Regex::new(f) {
            Ok(re) => re.is_match(text),
            Err(_) => text.contains(f.as_str()),
        };
        if matched {
            Some(f.as_str())
        } else {
            None
        }
    })
}

fn filtered_content(text: &str, filter_pattern: Option<&str>) -> String {
    if filter_pattern.is_some() {
        format!(
            "<span class=\"filtered-badge\">[FILTERED]</span> {}",
            collapsible_block(text, "")
        )
    } else {
        collapsible_block(text, "")
    }
}

pub fn render_system(json_str: &str, filters: &[String]) -> String {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    if let Some(s) = val.as_str() {
        let filter_match = matched_filter(s, filters);
        let row_class = if filter_match.is_some() {
            " class=\"filtered-row\""
        } else {
            ""
        };
        return format!(
            "<table><tr><th>Type</th><th>Content</th></tr><tr{}><td>text</td><td>{}</td></tr></table>",
            row_class,
            filtered_content(s, filter_match)
        );
    }

    if let Some(arr) = val.as_array() {
        let mut html = String::from("<table><tr><th>Type</th><th>Content</th></tr>");
        for block in arr {
            let btype = block.get("type").and_then(|t| t.as_str()).unwrap_or("text");
            let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
            let fallback;
            let text = if text.is_empty() {
                fallback = serde_json::to_string_pretty(block).unwrap_or_default();
                &fallback
            } else {
                text
            };
            let filter_match = matched_filter(text, filters);
            let row_class = if filter_match.is_some() {
                " class=\"filtered-row\""
            } else {
                ""
            };
            let cache_info = block
                .get("cache_control")
                .and_then(|c| c.get("type"))
                .and_then(|t| t.as_str())
                .map(|t| format!(" (cache: {})", html_escape(t)))
                .unwrap_or_default();
            html.push_str(&format!(
                "<tr{}><td>{}{}</td><td>{}</td></tr>",
                row_class,
                html_escape(btype),
                cache_info,
                filtered_content(text, filter_match)
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
