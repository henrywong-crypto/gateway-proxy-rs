use common::models::ProxyRequest;
use leptos::prelude::*;

use crate::collapsible_block;

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
