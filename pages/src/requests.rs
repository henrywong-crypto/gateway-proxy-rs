use common::models::{ProxyRequest, Session};
use leptos::{either::Either, prelude::*};
use std::collections::HashMap;
use templates::{pagination_nav, Breadcrumb, NavLink, Page, Pagination};

pub fn render_requests_view(
    session: &Session,
    requests: &[ProxyRequest],
    auto_refresh: bool,
    pagination: &Pagination,
) -> String {
    let session = session.clone();
    let requests = requests.to_vec();
    let total = pagination.total_items;

    let refresh_href = if auto_refresh {
        format!("/_dashboard/sessions/{}/requests?refresh=off", session.id)
    } else {
        format!("/_dashboard/sessions/{}/requests?refresh=on", session.id)
    };
    let refresh_label = if auto_refresh {
        "Disable auto-refresh"
    } else {
        "Enable auto-refresh"
    };

    let nav_top = pagination_nav(pagination);
    let nav_bottom = pagination_nav(pagination);

    let content = view! {
        {if auto_refresh {
            Some(view! { <meta http-equiv="refresh" content="3"/> })
        } else {
            None
        }}
        <h2>"Requests"</h2>
        <p>{format!("Total: {}", total)}</p>
        <a href={refresh_href}>{refresh_label}</a>
        {nav_top}
        {if requests.is_empty() {
            Either::Left(view! {
                <p>"No requests yet."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"ID"</th>
                        <th>"Method"</th>
                        <th>"Path"</th>
                        <th>"Model"</th>
                        <th>"Time"</th>
                        <th>"Messages"</th>
                        <th>"Last Message"</th>
                        <th>"Response"</th>
                        <th>"Last Block"</th>
                    </tr>
                    {requests.into_iter().map(|r| {
                        let detail_href = format!("/_dashboard/sessions/{}/requests/{}", r.session_id, r.id);
                        let messages_href = format!("/_dashboard/sessions/{}/requests/{}/messages", r.session_id, r.id);
                        let sse_href = format!("/_dashboard/sessions/{}/requests/{}/response_sse", r.session_id, r.id);
                        let (msg_count, preview) = get_message_preview(&r);
                        let (block_count, response_summary) = get_response_summary(&r);
                        let model = r.model.clone().unwrap_or_default();
                        let id_str = r.id.to_string();
                        view! {
                            <tr>
                                <td><a href={detail_href}>{id_str}</a></td>
                                <td>{r.method}</td>
                                <td>{r.path}</td>
                                <td>{model}</td>
                                <td>{r.timestamp}</td>
                                <td><a href={messages_href}>{msg_count}</a></td>
                                <td>{preview}</td>
                                <td><a href={sse_href}>{block_count}</a></td>
                                <td>{response_summary}</td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        }}
        {nav_bottom}
    };

    Page {
        title: format!("Gateway Proxy - Session {} - Requests", session.name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::link(
                format!("Session {}", session.name),
                format!("/_dashboard/sessions/{}", session.id),
            ),
            Breadcrumb::current("Requests"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}

/// Extract a preview string from a single content block.
fn extract_block_preview(block: &serde_json::Value) -> String {
    match block.get("type").and_then(|t| t.as_str()) {
        Some("text") => block
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string(),
        Some("tool_use") => format!(
            "tool_use: {}",
            block.get("name").and_then(|n| n.as_str()).unwrap_or("")
        ),
        Some("tool_result") => {
            let content_preview = if let Some(s) = block.get("content").and_then(|c| c.as_str()) {
                s.to_string()
            } else if let Some(arr) = block.get("content").and_then(|c| c.as_array()) {
                arr.iter()
                    .filter_map(|b| {
                        if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                            b.get("text").and_then(|t| t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                String::new()
            };
            if content_preview.is_empty() {
                "tool_result".to_string()
            } else {
                let preview = content_preview.replace('\n', " ");
                if preview.len() > 60 {
                    let truncated: String = preview.chars().take(60).collect();
                    format!("tool_result: {}...", truncated)
                } else {
                    format!("tool_result: {}", preview)
                }
            }
        }
        Some(t) => t.to_string(),
        None => String::new(),
    }
}

fn get_message_preview(proxy_request: &ProxyRequest) -> (String, String) {
    let Some(ref msgs_str) = proxy_request.messages_json else {
        return (String::new(), String::new());
    };
    let Ok(msgs) = serde_json::from_str::<Vec<serde_json::Value>>(msgs_str) else {
        return (String::new(), String::new());
    };
    if msgs.is_empty() {
        return (String::new(), String::new());
    }
    let count = format!("{}", msgs.len());
    let last = match msgs
        .iter()
        .rfind(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
    {
        Some(m) => m,
        None => match msgs.last() {
            Some(m) => m,
            None => return (count, String::new()),
        },
    };

    let content = &last["content"];
    let preview = if let Some(s) = content.as_str() {
        s.to_string()
    } else if let Some(arr) = content.as_array() {
        if let Some(block) = arr.last() {
            extract_block_preview(block)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let preview = preview.replace('\n', " ");
    if preview.len() > 80 {
        let truncated: String = preview.chars().take(80).collect();
        (count, format!("{}...", truncated))
    } else {
        (count, preview)
    }
}

/// Accumulate block metadata (types, names, text) from SSE events.
fn accumulate_sse_block_metadata(
    sse_events: &[serde_json::Value],
) -> (
    HashMap<i64, String>,
    HashMap<i64, String>,
    HashMap<i64, String>,
) {
    let mut block_types: HashMap<i64, String> = HashMap::new();
    let mut block_names: HashMap<i64, String> = HashMap::new();
    let mut block_text: HashMap<i64, String> = HashMap::new();

    for event in sse_events {
        let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
        let data = &event["data"];

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
                        let json = delta
                            .get("partial_json")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        block_text.entry(index).or_default().push_str(json);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    (block_types, block_names, block_text)
}

/// Format the last block's summary string from accumulated metadata.
fn format_last_block_summary(
    block_types: &HashMap<i64, String>,
    block_names: &HashMap<i64, String>,
    block_text: &HashMap<i64, String>,
) -> String {
    let last_index = {
        let mut indices: Vec<i64> = block_types.keys().copied().collect();
        indices.sort();
        indices.last().copied()
    };

    let Some(index) = last_index else {
        return String::new();
    };

    let btype = block_types.get(&index).map(|s| s.as_str()).unwrap_or("");
    let name = block_names.get(&index).map(|s| s.as_str()).unwrap_or("");
    let text = block_text.get(&index).map(|s| s.as_str()).unwrap_or("");

    match btype {
        "tool_use" => {
            let preview = text.replace('\n', " ");
            if !name.is_empty() && !preview.is_empty() {
                let short: String = preview.chars().take(40).collect();
                if preview.len() > 40 {
                    format!("{}({}): {}...", btype, name, short)
                } else {
                    format!("{}({}): {}", btype, name, short)
                }
            } else if !name.is_empty() {
                format!("{}({})", btype, name)
            } else {
                "tool_use".to_string()
            }
        }
        "thinking" => {
            let preview = text.replace('\n', " ");
            if preview.len() > 40 {
                let truncated: String = preview.chars().take(40).collect();
                format!("thinking: {}...", truncated)
            } else if preview.is_empty() {
                "thinking".to_string()
            } else {
                format!("thinking: {}", preview)
            }
        }
        _ => {
            let preview = text.replace('\n', " ");
            if preview.len() > 60 {
                let truncated: String = preview.chars().take(60).collect();
                format!("{}...", truncated)
            } else if !preview.is_empty() {
                preview
            } else {
                String::new()
            }
        }
    }
}

fn get_response_summary(proxy_request: &ProxyRequest) -> (String, String) {
    let Some(ref events_json) = proxy_request.response_events_json else {
        return match proxy_request.response_status {
            Some(status) => (String::new(), format!("{}", status)),
            None => (String::new(), String::new()),
        };
    };
    let Ok(sse_events) = serde_json::from_str::<Vec<serde_json::Value>>(events_json) else {
        return (String::new(), String::new());
    };

    let (block_types, block_names, block_text) = accumulate_sse_block_metadata(&sse_events);

    let num_blocks = block_types.len();
    if num_blocks == 0 {
        return (String::new(), String::new());
    }

    let summary = format_last_block_summary(&block_types, &block_names, &block_text);
    let count = format!("{}", num_blocks);
    (count, summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_block_preview tests ---

    #[test]
    fn extract_block_preview_text() {
        let block = serde_json::json!({"type": "text", "text": "Hello world"});
        assert_eq!(extract_block_preview(&block), "Hello world");
    }

    #[test]
    fn extract_block_preview_tool_use() {
        let block = serde_json::json!({"type": "tool_use", "name": "WebFetch"});
        assert_eq!(extract_block_preview(&block), "tool_use: WebFetch");
    }

    #[test]
    fn extract_block_preview_tool_result_string_content() {
        let block = serde_json::json!({"type": "tool_result", "content": "Result text"});
        assert_eq!(extract_block_preview(&block), "tool_result: Result text");
    }

    #[test]
    fn extract_block_preview_tool_result_array_content() {
        let block = serde_json::json!({
            "type": "tool_result",
            "content": [
                {"type": "text", "text": "first"},
                {"type": "text", "text": "second"},
            ]
        });
        assert_eq!(extract_block_preview(&block), "tool_result: first second");
    }

    #[test]
    fn extract_block_preview_tool_result_empty_content() {
        let block = serde_json::json!({"type": "tool_result"});
        assert_eq!(extract_block_preview(&block), "tool_result");
    }

    #[test]
    fn extract_block_preview_tool_result_truncated() {
        let long_text = "a".repeat(100);
        let block = serde_json::json!({"type": "tool_result", "content": long_text});
        let result = extract_block_preview(&block);
        assert!(result.starts_with("tool_result: "));
        assert!(result.ends_with("..."));
    }

    #[test]
    fn extract_block_preview_unknown_type() {
        let block = serde_json::json!({"type": "image"});
        assert_eq!(extract_block_preview(&block), "image");
    }

    #[test]
    fn extract_block_preview_no_type() {
        let block = serde_json::json!({"text": "hello"});
        assert_eq!(extract_block_preview(&block), "");
    }

    // --- accumulate_sse_block_metadata tests ---

    #[test]
    fn accumulate_sse_block_metadata_empty() {
        let events: Vec<serde_json::Value> = vec![];
        let (types, names, text) = accumulate_sse_block_metadata(&events);
        assert!(types.is_empty());
        assert!(names.is_empty());
        assert!(text.is_empty());
    }

    #[test]
    fn accumulate_sse_block_metadata_text_block() {
        let events = vec![
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "text"}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "text_delta", "text": "Hello "}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "text_delta", "text": "World"}}
            }),
        ];
        let (types, names, text) = accumulate_sse_block_metadata(&events);
        assert_eq!(types.get(&0).unwrap(), "text");
        assert!(!names.contains_key(&0));
        assert_eq!(text.get(&0).unwrap(), "Hello World");
    }

    #[test]
    fn accumulate_sse_block_metadata_tool_use_block() {
        let events = vec![
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 1, "content_block": {"type": "tool_use", "name": "WebFetch"}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"url\":"}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "input_json_delta", "partial_json": " \"test\"}"}}
            }),
        ];
        let (types, names, text) = accumulate_sse_block_metadata(&events);
        assert_eq!(types.get(&1).unwrap(), "tool_use");
        assert_eq!(names.get(&1).unwrap(), "WebFetch");
        assert_eq!(text.get(&1).unwrap(), "{\"url\": \"test\"}");
    }

    #[test]
    fn accumulate_sse_block_metadata_multiple_blocks() {
        let events = vec![
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "thinking"}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "thinking_delta", "thinking": "hmm"}}
            }),
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 1, "content_block": {"type": "text"}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "text_delta", "text": "result"}}
            }),
        ];
        let (types, _names, text) = accumulate_sse_block_metadata(&events);
        assert_eq!(types.len(), 2);
        assert_eq!(types.get(&0).unwrap(), "thinking");
        assert_eq!(types.get(&1).unwrap(), "text");
        assert_eq!(text.get(&0).unwrap(), "hmm");
        assert_eq!(text.get(&1).unwrap(), "result");
    }

    // --- format_last_block_summary tests ---

    #[test]
    fn format_last_block_summary_empty_maps() {
        let types = HashMap::new();
        let names = HashMap::new();
        let text = HashMap::new();
        assert_eq!(format_last_block_summary(&types, &names, &text), "");
    }

    #[test]
    fn format_last_block_summary_text_block() {
        let mut types = HashMap::new();
        let names = HashMap::new();
        let mut text = HashMap::new();
        types.insert(0, "text".to_string());
        text.insert(0, "Hello world".to_string());
        assert_eq!(
            format_last_block_summary(&types, &names, &text),
            "Hello world"
        );
    }

    #[test]
    fn format_last_block_summary_text_truncated() {
        let mut types = HashMap::new();
        let names = HashMap::new();
        let mut text = HashMap::new();
        types.insert(0, "text".to_string());
        text.insert(0, "a".repeat(100));
        let result = format_last_block_summary(&types, &names, &text);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn format_last_block_summary_tool_use_with_name() {
        let mut types = HashMap::new();
        let mut names = HashMap::new();
        let mut text = HashMap::new();
        types.insert(0, "tool_use".to_string());
        names.insert(0, "WebFetch".to_string());
        text.insert(0, "{\"url\": \"test\"}".to_string());
        let result = format_last_block_summary(&types, &names, &text);
        assert!(result.starts_with("tool_use(WebFetch): "));
    }

    #[test]
    fn format_last_block_summary_tool_use_name_only() {
        let mut types = HashMap::new();
        let mut names = HashMap::new();
        let text = HashMap::new();
        types.insert(0, "tool_use".to_string());
        names.insert(0, "WebSearch".to_string());
        assert_eq!(
            format_last_block_summary(&types, &names, &text),
            "tool_use(WebSearch)"
        );
    }

    #[test]
    fn format_last_block_summary_tool_use_no_name() {
        let mut types = HashMap::new();
        let names = HashMap::new();
        let text = HashMap::new();
        types.insert(0, "tool_use".to_string());
        assert_eq!(format_last_block_summary(&types, &names, &text), "tool_use");
    }

    #[test]
    fn format_last_block_summary_thinking() {
        let mut types = HashMap::new();
        let names = HashMap::new();
        let mut text = HashMap::new();
        types.insert(0, "thinking".to_string());
        text.insert(0, "Let me think".to_string());
        assert_eq!(
            format_last_block_summary(&types, &names, &text),
            "thinking: Let me think"
        );
    }

    #[test]
    fn format_last_block_summary_thinking_empty() {
        let mut types = HashMap::new();
        let names = HashMap::new();
        let text = HashMap::new();
        types.insert(0, "thinking".to_string());
        assert_eq!(format_last_block_summary(&types, &names, &text), "thinking");
    }

    #[test]
    fn format_last_block_summary_uses_highest_index() {
        let mut types = HashMap::new();
        let names = HashMap::new();
        let mut text = HashMap::new();
        types.insert(0, "thinking".to_string());
        text.insert(0, "thought".to_string());
        types.insert(1, "text".to_string());
        text.insert(1, "output".to_string());
        // Should use index 1 (highest)
        assert_eq!(format_last_block_summary(&types, &names, &text), "output");
    }
}
