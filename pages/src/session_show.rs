use common::models::Session;
use leptos::prelude::*;
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

fn render_copy_link(url: &str) -> impl IntoView {
    let onclick = format!("navigator.clipboard.writeText('{}')", url);
    let url = url.to_string();
    view! {
        {url}
        " " <a href="javascript:void(0)" onclick={onclick}>"Copy"</a>
    }
}

pub fn render_session_view(session: &Session, port: u16, profile_name: Option<&str>) -> String {
    let proxy_url = format!("http://localhost:{}/_proxy/{}/", port, session.id);
    let bedrock_url = format!("http://localhost:{}/_bedrock/{}/", port, session.id);

    let mut info_rows = vec![
        InfoRow::new("Name", &session.name),
        InfoRow::view("Proxy URL", render_copy_link(&proxy_url)),
        InfoRow::view("Bedrock URL", render_copy_link(&bedrock_url)),
        InfoRow::new("Target", &session.target_url),
    ];

    if let Some(name) = profile_name {
        info_rows.push(InfoRow::new("Filter Profile", name));
    }

    Page {
        title: format!("Gateway Proxy - Session {}", session.name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::current(format!("Session {}", session.name)),
        ],
        nav_links: vec![
            NavLink::new(
                "Edit Session",
                format!("/_dashboard/sessions/{}/edit", session.id),
            ),
            NavLink::back(),
        ],
        info_rows,
        content: (),
        subpages: vec![
            Subpage::new(
                "Requests",
                format!("/_dashboard/sessions/{}/requests", session.id),
                session.request_count,
            ),
            Subpage::new(
                "Error Injection",
                format!("/_dashboard/sessions/{}/error-inject", session.id),
                if session.error_inject.as_deref().unwrap_or("").is_empty() {
                    "off"
                } else {
                    "on"
                },
            ),
            Subpage::new(
                "Intercept",
                format!("/_dashboard/sessions/{}/intercept", session.id),
                format!(
                    "fetch: {}",
                    if session.webfetch_intercept {
                        "on"
                    } else {
                        "off"
                    },
                ),
            ),
        ],
    }
    .render()
}
