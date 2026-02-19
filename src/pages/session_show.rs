use leptos::prelude::*;

use crate::models::Session;
use crate::pages::page_layout;

pub fn render_session_show(session: &Session, port: u16) -> String {
    let session = session.clone();
    let session_name = session.name.clone();
    let proxy_url = format!("http://localhost:{}/_proxy/{}/", port, session.id);
    let requests_href = format!("/_dashboard/sessions/{}/requests", session.id);
    let edit_href = format!("/_dashboard/sessions/{}/edit", session.id);

    let body = view! {
        <h1>
            <a href="/_dashboard">"Home"</a>
            " / "
            <a href="/_dashboard/sessions">"Sessions"</a>
            " / "
            {format!("Session {}", session.name)}
        </h1>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href={edit_href}>"Edit Session"</a></td></tr>
            <tr><td><a href="/_dashboard/sessions">"Back"</a></td></tr>
        </table>
        <h2>"Info"</h2>
        <table>
            <tr>
                <td>"Name"</td>
                <td>{session.name.clone()}</td>
            </tr>
            <tr>
                <td>"Proxy URL"</td>
                <td>{proxy_url}</td>
            </tr>
            <tr>
                <td>"Target"</td>
                <td>{session.target_url}</td>
            </tr>
        </table>
        <h2>"Requests"</h2>
        <table>
            <tr><td><a href={requests_href}>"Requests"</a></td></tr>
        </table>
    };

    page_layout(&format!("Gateway Proxy - Session {}", session_name), body.to_html())
}
