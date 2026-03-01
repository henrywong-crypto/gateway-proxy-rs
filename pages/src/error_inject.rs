use common::{error_inject::ERROR_TYPES, models::Session};
use leptos::{either::Either, prelude::*};
use templates::{Breadcrumb, NavLink, Page};

pub fn render_error_inject_view(session: &Session) -> String {
    let session_id = session.id.to_string();
    let form_action = format!("/_dashboard/sessions/{}/error-inject", session_id);
    let clear_action = format!("/_dashboard/sessions/{}/error-inject/clear", session_id);

    let active_key = session.error_inject.clone().unwrap_or_default();
    let is_active = !active_key.is_empty();

    let active_label = common::error_inject::find_by_key(&active_key)
        .map(|e| e.label)
        .unwrap_or("unknown");

    let rows: Vec<_> = ERROR_TYPES
        .iter()
        .map(|error| {
            let is_selected = error.key == active_key;
            let row_class = if is_selected { "filtered-row" } else { "" };
            let label = error.label.to_string();
            let data_json = error.data_json.to_string();
            let form_action = form_action.clone();
            let key = error.key.to_string();
            let badge = if is_selected {
                Either::Left(view! { " " <span class="filtered-badge">"[ACTIVE]"</span> })
            } else {
                Either::Right(())
            };
            view! {
                <tr class={row_class}>
                    <td>{label}{badge}</td>
                    <td><pre>{data_json}</pre></td>
                    <td>
                        <form method="POST" action={form_action}>
                            <input type="hidden" name="error_type" value={key}/>
                            <button type="submit">"Inject"</button>
                        </form>
                    </td>
                </tr>
            }
        })
        .collect();

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
        <table>
            <tr><th>"Error Type"</th><th>"SSE Payload"</th><th></th></tr>
            {rows}
        </table>
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
