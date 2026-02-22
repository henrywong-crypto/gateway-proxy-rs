use templates::{Breadcrumb, Page, Subpage};

pub fn render_home(session_count: i64, profile_count: i64) -> String {
    Page {
        title: "Gateway Proxy - Home".to_string(),
        breadcrumbs: vec![Breadcrumb::current("Home")],
        subpages: vec![
            Subpage::new("Sessions", "/_dashboard/sessions", session_count),
            Subpage::new("Profiles", "/_dashboard/filters", profile_count),
        ],
        ..Default::default()
    }
    .render()
}
