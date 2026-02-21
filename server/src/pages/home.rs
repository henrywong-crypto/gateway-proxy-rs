use leptos::prelude::*;

use crate::pages::page_layout;

pub fn render_home() -> String {
    let body = view! {
        <h1>"Home"</h1>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="/_dashboard/sessions">"Sessions"</a></td></tr>
            <tr><td><a href="/_dashboard/filters">"System Filters"</a></td></tr>
        </table>
    };
    page_layout("Gateway Proxy - Home", body.to_html())
}
