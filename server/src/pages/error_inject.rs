use leptos::either::Either;
use leptos::prelude::*;
use templates::{html_escape, Breadcrumb, NavLink, Page};

use common::error_inject::ERROR_TYPES;
use common::models::Session;

pub fn render_error_inject(session: &Session) -> String {
    let session_id = session.id.to_string();
    let form_action = format!("/_dashboard/sessions/{}/error-inject", session_id);
    let clear_action = format!("/_dashboard/sessions/{}/error-inject/clear", session_id);

    let active_key = session.error_inject.clone().unwrap_or_default();
    let is_active = !active_key.is_empty();

    let active_label = common::error_inject::label_for_key(&active_key).unwrap_or("unknown");

    let mut rows_html = String::new();
    for error in ERROR_TYPES {
        let is_selected = error.key == active_key;
        let row_class = if is_selected {
            " class=\"filtered-row\""
        } else {
            ""
        };
        let badge = if is_selected {
            " <span class=\"filtered-badge\">[ACTIVE]</span>"
        } else {
            ""
        };
        rows_html.push_str(&format!(
            "<tr{}><td>{}{}</td><td><pre>{}</pre></td><td><form method=\"POST\" action=\"{}\"><input type=\"hidden\" name=\"error_type\" value=\"{}\"/><button type=\"submit\">Inject</button></form></td></tr>",
            row_class,
            html_escape(error.label),
            badge,
            html_escape(error.data_json),
            html_escape(&form_action),
            html_escape(error.key),
        ));
    }

    let table_html = format!(
        "<table><tr><th>Error Type</th><th>SSE Payload</th><th></th></tr>{}</table>",
        rows_html
    );

    let content = view! {
        {if is_active {
            Either::Left(view! {
                <h2>"Active Injection"</h2>
                <p>
                    "All requests on this session are returning: "
                    <strong>{active_label.to_string()}</strong>
                    " "
                    <form method="POST" action={clear_action}>
                        <button type="submit">"Disable"</button>
                    </form>
                </p>
            })
        } else {
            Either::Right(view! {
                <h2>"No Active Injection"</h2>
                <p>"All requests pass through normally."</p>
            })
        }}

        <h2>"Error Types"</h2>
        <div inner_html={table_html}/>
    };

    Page {
        title: format!("Gateway Proxy - Session {} - Error Injection", session.name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::link(
                format!("Session {}", session.name),
                format!("/_dashboard/sessions/{}", session_id),
            ),
            Breadcrumb::current("Error Injection"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
