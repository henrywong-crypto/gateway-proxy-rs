use common::models::{ProxyRequest, Session};
use leptos::{either::Either, prelude::*};
use std::collections::HashMap;
use templates::{Breadcrumb, NavLink, Page};

pub fn render_requests_index(
    session: &Session,
    requests: &[ProxyRequest],
    auto_refresh: bool,
) -> String {
    let session = session.clone();
    let requests = requests.to_vec();
    let total = requests.len();

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

    let content = view! {
        {if auto_refresh {
            Some(view! { <meta http-equiv="refresh" content="3"/> })
        } else {
            None
        }}
        <h2>"Requests"</h2>
        <p>{format!("Total: {}", total)}</p>
        <a href={refresh_href}>{refresh_label}</a>
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

fn get_message_preview(r: &ProxyRequest) -> (String, String) {
    let Some(ref msgs_str) = r.messages_json else {
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
                    let content_preview =
                        if let Some(s) = block.get("content").and_then(|c| c.as_str()) {
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

fn get_response_summary(r: &ProxyRequest) -> (String, String) {
    let Some(ref events_json) = r.response_events_json else {
        return match r.response_status {
            Some(status) => (String::new(), format!("{}", status)),
            None => (String::new(), String::new()),
        };
    };
    let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(events_json) else {
        return (String::new(), String::new());
    };

    let mut block_types: HashMap<i64, String> = HashMap::new();
    let mut block_names: HashMap<i64, String> = HashMap::new();
    let mut block_text: HashMap<i64, String> = HashMap::new();

    for event in &events {
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

    // Show only the last content block's summary, prefixed with block count
    let num_blocks = block_types.len();
    let last_index = {
        let mut indices: Vec<i64> = block_types.keys().copied().collect();
        indices.sort();
        indices.last().copied()
    };

    let Some(index) = last_index else {
        return (String::new(), String::new());
    };

    let btype = block_types.get(&index).map(|s| s.as_str()).unwrap_or("");
    let name = block_names.get(&index).map(|s| s.as_str()).unwrap_or("");
    let text = block_text.get(&index).map(|s| s.as_str()).unwrap_or("");

    let summary = match btype {
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
    };

    let count = format!("{}", num_blocks);
    (count, summary)
}
