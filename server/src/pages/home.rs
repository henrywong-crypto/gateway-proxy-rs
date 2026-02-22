use leptos::prelude::*;

use crate::pages::page_layout;

pub fn render_home(session_count: i64, profile_count: i64) -> String {
    let body = view! {
        <h1>"Home"</h1>
        <h2>"Subpages"</h2>
        <table>
            <tr>
                <th>"Page"</th>
                <th>"Count"</th>
            </tr>
            <tr>
                <td><a href="/_dashboard/sessions">"Sessions"</a></td>
                <td>{session_count}</td>
            </tr>
            <tr>
                <td><a href="/_dashboard/filters">"Profiles"</a></td>
                <td>{profile_count}</td>
            </tr>
        </table>
    };
    page_layout("Gateway Proxy - Home", body.to_html())
}
