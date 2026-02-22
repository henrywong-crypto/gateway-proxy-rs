use crate::pages::{collapsible_block, html_escape};
use ::common::models::ProxyRequest;

pub fn render_kv_table(json_str: &str) -> String {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    let Some(obj) = val.as_object() else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    let mut html = String::from("<table><tr><th>Key</th><th>Value</th></tr>");
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

pub fn render_response_headers(req: &ProxyRequest) -> String {
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
