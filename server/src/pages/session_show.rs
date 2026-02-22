use leptos::prelude::*;

use crate::pages::{html_escape, page_layout};
use common::models::Session;

fn copy_link_html(text: &str) -> String {
    format!(
        r#" <a href="javascript:void(0)" onclick="navigator.clipboard.writeText('{}')">Copy</a>"#,
        html_escape(text)
    )
}

pub fn render_session_show(session: &Session, port: u16) -> String {
    let session = session.clone();
    let session_name = session.name.clone();
    let proxy_url = format!("http://localhost:{}/_proxy/{}/", port, session.id);
    let bedrock_url = format!("http://localhost:{}/_bedrock/{}/", port, session.id);
    let requests_href = format!("/_dashboard/sessions/{}/requests", session.id);
    let edit_href = format!("/_dashboard/sessions/{}/edit", session.id);

    let proxy_copy = copy_link_html(&proxy_url);
    let bedrock_copy = copy_link_html(&bedrock_url);

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
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>"Info"</h2>
        <table>
            <tr>
                <td>"Name"</td>
                <td>{session.name.clone()}</td>
            </tr>
            <tr>
                <td>"Proxy URL"</td>
                <td>{proxy_url.clone()} <span inner_html={proxy_copy}/></td>
            </tr>
            <tr>
                <td>"Bedrock URL"</td>
                <td>{bedrock_url.clone()} <span inner_html={bedrock_copy}/></td>
            </tr>
            <tr>
                <td>"Target"</td>
                <td>{session.target_url}</td>
            </tr>
        </table>
        <h2>"Subpages"</h2>
        <table>
            <tr>
                <th>"Page"</th>
                <th>"Count"</th>
            </tr>
            <tr>
                <td><a href={requests_href}>"Requests"</a></td>
                <td>{session.request_count}</td>
            </tr>
        </table>
    };

    page_layout(
        &format!("Gateway Proxy - Session {}", session_name),
        body.to_html(),
    )
}
