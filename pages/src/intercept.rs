use common::models::Session;
use templates::{Breadcrumb, NavLink, Page, Subpage};

pub fn render_intercept_view(session: &Session, pending_count: usize) -> String {
    let session_id = session.id.to_string();

    Page {
        title: format!("Gateway Proxy - Session {} - Tool Intercept", session.name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::link(
                format!("Session {}", session.name),
                format!("/_dashboard/sessions/{}", session_id),
            ),
            Breadcrumb::current("Tool Intercept"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content: (),
        subpages: vec![
            Subpage::new(
                "WebFetch Intercept",
                format!(
                    "/_dashboard/sessions/{}/tool-intercept/webfetch",
                    session_id
                ),
                if session.webfetch_intercept {
                    "on"
                } else {
                    "off"
                },
            ),
            Subpage::new(
                "Pending Approvals",
                format!(
                    "/_dashboard/sessions/{}/tool-intercept/approvals",
                    session_id
                ),
                pending_count,
            ),
        ],
    }
    .render()
}
