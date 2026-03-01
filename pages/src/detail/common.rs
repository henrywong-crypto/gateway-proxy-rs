use common::models::ProxyRequest;
use leptos::prelude::*;
use std::collections::HashMap;
use templates::Subpage;

use crate::collapsible_block;

use super::{
    messages::render_messages, sse::render_response_sse, system::render_system, tools::render_tools,
};

pub fn render_kv_table(json_str: &str) -> AnyView {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        let s = json_str.to_string();
        return view! { <pre>{s}</pre> }.into_any();
    };

    let Some(obj) = val.as_object() else {
        let s = json_str.to_string();
        return view! { <pre>{s}</pre> }.into_any();
    };

    let rows: Vec<AnyView> = obj
        .iter()
        .map(|(k, v)| {
            let val_str = if v.is_string() {
                v.as_str().unwrap_or("").to_string()
            } else {
                serde_json::to_string_pretty(v).unwrap_or_default()
            };
            let k = k.clone();
            let cb = collapsible_block(&val_str, "");
            view! {
                <tr><td>{k}</td><td>{cb}</td></tr>
            }
            .into_any()
        })
        .collect();

    view! {
        <table>
            <tr><th>"Key"</th><th>"Value"</th></tr>
            {rows}
        </table>
    }
    .into_any()
}

pub fn render_response_headers(req: &ProxyRequest) -> AnyView {
    let status_view: AnyView = if let Some(status) = req.response_status {
        let status_str = status.to_string();
        view! {
            <div><strong>"Status:"</strong>" "{status_str}</div>
        }
        .into_any()
    } else {
        ().into_any()
    };

    let headers_view: AnyView = if let Some(ref headers) = req.response_headers_json {
        render_kv_table(headers)
    } else {
        ().into_any()
    };

    let has_content = req.response_status.is_some() || req.response_headers_json.is_some();

    if has_content {
        view! {
            {status_view}
            {headers_view}
        }
        .into_any()
    } else {
        view! { <p>"No response headers."</p> }.into_any()
    }
}

pub fn count_json_array(json: Option<&str>) -> Option<usize> {
    json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v.as_array().map(|a| a.len()))
}

pub fn count_json_object(json: Option<&str>) -> Option<usize> {
    json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v.as_object().map(|o| o.len()))
}

pub fn count_json_items(json: Option<&str>) -> Option<usize> {
    json.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| {
            v.as_array()
                .map(|a| a.len())
                .or_else(|| v.as_object().map(|o| o.len()))
        })
}

/// Build the standard subpage definitions for a request detail view.
/// When `include_webfetch` is true, includes the WebFetch Intercept subpage.
pub fn build_request_subpage_defs(
    req: &ProxyRequest,
    base_url: &str,
    include_webfetch: bool,
) -> Vec<Subpage> {
    let has_response = req.response_body.is_some() || req.response_events_json.is_some();

    let mut subpage_defs: Vec<(&str, &str, bool, String)> = vec![
        (
            "messages",
            "Messages",
            req.messages_json.is_some(),
            count_json_array(req.messages_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "system",
            "System",
            req.system_json.is_some(),
            count_json_array(req.system_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "tools",
            "Tools",
            req.tools_json.is_some(),
            count_json_array(req.tools_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "params",
            "Params",
            req.params_json.is_some(),
            count_json_object(req.params_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        ("full_json", "Full JSON", true, String::new()),
        (
            "response_sse",
            "Response SSE",
            req.response_events_json.is_some(),
            count_json_array(req.response_events_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "headers",
            "Request Headers",
            true,
            count_json_object(req.headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        (
            "response_headers",
            "Response Headers",
            has_response,
            count_json_object(req.response_headers_json.as_deref())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
    ];

    if include_webfetch {
        let has_ws = req.webfetch_first_response_events_json.is_some()
            || req.webfetch_followup_body_json.is_some();
        subpage_defs.push((
            "webfetch_intercept",
            "WebFetch Intercept",
            has_ws,
            String::new(),
        ));
    }

    subpage_defs
        .into_iter()
        .filter(|(_, _, available, _)| *available)
        .map(|(key, label, _, count)| Subpage::new(label, format!("{}/{}", base_url, key), count))
        .collect()
}

/// Rendered detail page content — controls, main content, and total count views.
pub struct DetailPageContent {
    pub controls_view: AnyView,
    pub content_view: AnyView,
    pub total_view: AnyView,
}

/// Render the content for a request detail subpage (Messages, System, Tools, etc.).
pub fn render_detail_page_content(
    req: &ProxyRequest,
    base_url: &str,
    page: &str,
    query: &HashMap<String, String>,
    filters: &[String],
    keep_tool_pairs: i64,
) -> DetailPageContent {
    let truncate = query.get("truncate").map(|v| v.as_str()) != Some("off");
    let order = query
        .get("order")
        .cloned()
        .unwrap_or_else(|| "desc".to_string());

    let mut controls_view: AnyView = ().into_any();

    let content_view: AnyView = match page {
        "messages" => {
            if let Some(ref json_str) = req.messages_json {
                let toggle_order = if order == "desc" { "asc" } else { "desc" };
                let toggle_href = format!("{}/messages?order={}", base_url, toggle_order);
                let showing = if order == "desc" {
                    "newest first"
                } else {
                    "oldest first"
                };
                let switch_to = if order == "desc" {
                    "oldest first"
                } else {
                    "newest first"
                };
                controls_view = view! {
                    <div>"Showing: "{showing}" | "<a href={toggle_href}>"Switch to "{switch_to}</a></div>
                }
                .into_any();
                render_messages(json_str, &order, keep_tool_pairs)
            } else {
                view! { <p>"No messages."</p> }.into_any()
            }
        }
        "system" => req
            .system_json
            .as_deref()
            .map(|s| render_system(s, filters))
            .unwrap_or_else(|| view! { <p>"No system prompt."</p> }.into_any()),
        "tools" => req
            .tools_json
            .as_deref()
            .map(|s| render_tools(s, filters))
            .unwrap_or_else(|| view! { <p>"No tools."</p> }.into_any()),
        "params" => req
            .params_json
            .as_deref()
            .map(render_kv_table)
            .unwrap_or_else(|| view! { <p>"No params."</p> }.into_any()),
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
                base_url,
                if truncate { "off" } else { "on" }
            );
            let toggle_label = if truncate {
                "Show full strings"
            } else {
                "Show truncated"
            };
            controls_view = view! {
                <a href={toggle_href}>{toggle_label}</a>
            }
            .into_any();
            let json = json.to_string();
            view! {
                <textarea readonly rows="30" cols="80" wrap="off">{json}</textarea>
            }
            .into_any()
        }
        "response_headers" => render_response_headers(req),
        "response_sse" => render_response_sse(req),
        _ => view! { <p>"Unknown tab"</p> }.into_any(),
    };

    let total_count = match page {
        "messages" => count_json_items(req.messages_json.as_deref()),
        "system" => count_json_items(req.system_json.as_deref()),
        "tools" => count_json_items(req.tools_json.as_deref()),
        "params" => count_json_items(req.params_json.as_deref()),
        "headers" => count_json_items(req.headers_json.as_deref()),
        "response_headers" => count_json_items(req.response_headers_json.as_deref()),
        "response_sse" => count_json_items(req.response_events_json.as_deref()),
        _ => None,
    };
    let total_view: AnyView = total_count
        .map(|n| {
            let n = n.to_string();
            view! { <p>"Total: "{n}</p> }.into_any()
        })
        .unwrap_or_else(|| ().into_any());

    DetailPageContent {
        controls_view,
        content_view,
        total_view,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- count_json_array tests ---

    #[test]
    fn count_json_array_none() {
        assert_eq!(count_json_array(None), None);
    }

    #[test]
    fn count_json_array_valid() {
        assert_eq!(count_json_array(Some("[1, 2, 3]")), Some(3));
    }

    #[test]
    fn count_json_array_empty() {
        assert_eq!(count_json_array(Some("[]")), Some(0));
    }

    #[test]
    fn count_json_array_object_returns_none() {
        assert_eq!(count_json_array(Some("{\"a\": 1}")), None);
    }

    #[test]
    fn count_json_array_invalid_json() {
        assert_eq!(count_json_array(Some("not json")), None);
    }

    // --- count_json_object tests ---

    #[test]
    fn count_json_object_none() {
        assert_eq!(count_json_object(None), None);
    }

    #[test]
    fn count_json_object_valid() {
        assert_eq!(count_json_object(Some("{\"a\": 1, \"b\": 2}")), Some(2));
    }

    #[test]
    fn count_json_object_empty() {
        assert_eq!(count_json_object(Some("{}")), Some(0));
    }

    #[test]
    fn count_json_object_array_returns_none() {
        assert_eq!(count_json_object(Some("[1, 2]")), None);
    }

    // --- count_json_items tests ---

    #[test]
    fn count_json_items_none() {
        assert_eq!(count_json_items(None), None);
    }

    #[test]
    fn count_json_items_array() {
        assert_eq!(count_json_items(Some("[1, 2]")), Some(2));
    }

    #[test]
    fn count_json_items_object() {
        assert_eq!(count_json_items(Some("{\"a\": 1}")), Some(1));
    }

    #[test]
    fn count_json_items_string_returns_none() {
        assert_eq!(count_json_items(Some("\"hello\"")), None);
    }

    #[test]
    fn count_json_items_number_returns_none() {
        assert_eq!(count_json_items(Some("42")), None);
    }
}
