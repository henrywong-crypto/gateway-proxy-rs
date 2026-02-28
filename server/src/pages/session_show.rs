use templates::{html_escape, Breadcrumb, InfoRow, NavLink, Page, Subpage};

use common::models::Session;

fn copy_link_html(text: &str) -> String {
    format!(
        r#" <a href="javascript:void(0)" onclick="navigator.clipboard.writeText('{}')">Copy</a>"#,
        html_escape(text)
    )
}

pub fn render_session_show(session: &Session, port: u16, profile_name: Option<&str>) -> String {
    let proxy_url = format!("http://localhost:{}/_proxy/{}/", port, session.id);
    let bedrock_url = format!("http://localhost:{}/_bedrock/{}/", port, session.id);

    let mut info_rows = vec![
        InfoRow::new("Name", &session.name),
        InfoRow::raw(
            "Proxy URL",
            format!("{}{}", html_escape(&proxy_url), copy_link_html(&proxy_url)),
        ),
        InfoRow::raw(
            "Bedrock URL",
            format!(
                "{}{}",
                html_escape(&bedrock_url),
                copy_link_html(&bedrock_url)
            ),
        ),
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
                "WebSearch Intercept",
                format!("/_dashboard/sessions/{}/websearch", session.id),
                format!(
                    "search: {}, fetch: {}",
                    if session.websearch_intercept {
                        "on"
                    } else {
                        "off"
                    },
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
