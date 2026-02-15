use leptos::either::Either;
use leptos::prelude::*;

use crate::models::{ProxyRequest, Session};
use crate::pages::page_layout;

pub fn render_requests_index(
    session: &Session,
    requests: &[ProxyRequest],
    auto_refresh: bool,
) -> String {
    let session = session.clone();
    let requests = requests.to_vec();
    let session_name = session.name.clone();

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

    let body = view! {
        {if auto_refresh {
            Some(view! { <meta http-equiv="refresh" content="3"/> })
        } else {
            None
        }}
        <h1>
            <a href="/_dashboard">"Home"</a>
            " / "
            <a href="/_dashboard/sessions">"Sessions"</a>
            " / "
            <a href={format!("/_dashboard/sessions/{}", session.id)}>{format!("Session {}", session.name)}</a>
            " / "
            "Requests"
        </h1>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href={format!("/_dashboard/sessions/{}", session.id)}>{"Back"}</a></td></tr>
        </table>
        <h2>"Requests"</h2>
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
                        <th>"Preview"</th>
                    </tr>
                    {requests.into_iter().map(|r| {
                        let detail_href = format!("/_dashboard/sessions/{}/requests/{}", r.session_id, r.id);
                        let preview = get_message_preview(&r);
                        let model = r.model.unwrap_or_default();
                        view! {
                            <tr>
                                <td><a href={detail_href}>{r.id}</a></td>
                                <td>{r.method}</td>
                                <td>{r.path}</td>
                                <td>{model}</td>
                                <td>{r.timestamp}</td>
                                <td>
                                    {preview}
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        }}
    };

    page_layout(&format!("Gateway Proxy - Session {} - Requests", session_name), body.to_html())
}

fn get_message_preview(r: &ProxyRequest) -> String {
    let Some(ref msgs_str) = r.messages_json else {
        return String::new();
    };
    let Ok(msgs) = serde_json::from_str::<Vec<serde_json::Value>>(msgs_str) else {
        return String::new();
    };
    if msgs.is_empty() {
        return String::new();
    }
    let last = msgs
        .iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .last()
        .unwrap_or(msgs.last().unwrap());

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
                Some("tool_result") => "tool_result".to_string(),
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
        format!("[{}] {}...", msgs.len(), truncated)
    } else {
        format!("[{}] {}", msgs.len(), preview)
    }
}
