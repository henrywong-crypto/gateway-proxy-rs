use leptos::either::Either;
use leptos::prelude::*;

use crate::pages::page_layout;
use common::models::{Session, SessionWithCount};

pub fn render_sessions_index(sessions: &[SessionWithCount]) -> String {
    let sessions = sessions.to_vec();
    let empty = sessions.is_empty();
    let body = view! {
        <h1>
            <a href="/_dashboard">"Home"</a>
            " / "
            "Sessions"
        </h1>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="/_dashboard/sessions/new">"New Session"</a></td></tr>
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>"Sessions"</h2>
        {if empty {
            Either::Left(view! {
                <p>"No sessions yet."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"ID"</th>
                        <th>"Name"</th>
                        <th>"Target URL"</th>
                        <th>"Requests"</th>
                        <th>"Created"</th>
                        <th></th>
                    </tr>
                    {sessions.into_iter().map(|s| {
                        let href = format!("/_dashboard/sessions/{}", s.id);
                        let clear_action = format!("/_dashboard/sessions/{}/clear", s.id);
                        let delete_action = format!("/_dashboard/sessions/{}/delete", s.id);
                        view! {
                            <tr>
                                <td><a href={href}>{s.id.clone()}</a></td>
                                <td>{s.name}</td>
                                <td>{s.target_url}</td>
                                <td>{s.request_count}</td>
                                <td>{s.created_at.clone().unwrap_or_default()}</td>
                                <td>
                                    <form method="POST" action={clear_action}>
                                        <button type="submit">"Clear"</button>
                                    </form>
                                    " "
                                    <form method="POST" action={delete_action}>
                                        <button type="submit">"Delete"</button>
                                    </form>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        }}
    };
    page_layout("Gateway Proxy - Sessions", body.to_html())
}

pub fn render_new_session() -> String {
    let body = view! {
        <h1>
            <a href="/_dashboard">"Home"</a>
            " / "
            <a href="/_dashboard/sessions">"Sessions"</a>
            " / "
            "New Session"
        </h1>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>"New Session"</h2>
        <form method="POST" action="/_dashboard/sessions/new">
            <table>
                <tr>
                    <td><label>"Name"</label></td>
                    <td><input type="text" name="name" required placeholder="my-session"/></td>
                </tr>
                <tr>
                    <td><label>"Target URL"</label></td>
                    <td><input type="text" name="target_url" required placeholder="https://api.example.com" size="40"/></td>
                </tr>
                <tr>
                    <td><label>"Disable TLS Verify"</label></td>
                    <td><input type="checkbox" name="tls_verify_disabled" value="1"/></td>
                </tr>
                <tr>
                    <td><label>"Authorization Header"</label></td>
                    <td><input type="text" name="auth_header" placeholder="Bearer sk-..." size="40"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Create"/></td>
                </tr>
            </table>
        </form>
    };
    page_layout("Gateway Proxy - New Session", body.to_html())
}

pub fn render_edit_session(session: &Session, port: u16) -> String {
    let session = session.clone();
    let session_name = session.name.clone();
    let edit_action = format!("/_dashboard/sessions/{}/edit", session.id);
    let proxy_url = format!("http://localhost:{}/_proxy/{}/", port, session.id);
    let requests_href = format!("/_dashboard/sessions/{}/requests", session.id);
    let back_href = format!("/_dashboard/sessions/{}", session.id);
    let tls_disabled = session.tls_verify_disabled;
    let auth_header_val = session.auth_header.clone().unwrap_or_default();

    let body = view! {
        <h1>
            <a href="/_dashboard">"Home"</a>
            " / "
            <a href="/_dashboard/sessions">"Sessions"</a>
            " / "
            <a href={back_href.clone()}>{format!("Session {}", session_name)}</a>
            " / "
            "Edit"
        </h1>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>"Info"</h2>
        <table>
            <tr>
                <td>"Proxy URL"</td>
                <td>{proxy_url}</td>
            </tr>
        </table>
        <h2>"Edit Session"</h2>
        <form method="POST" action={edit_action}>
            <table>
                <tr>
                    <td><label>"Name"</label></td>
                    <td><input type="text" name="name" required value={session.name}/></td>
                </tr>
                <tr>
                    <td><label>"Target URL"</label></td>
                    <td><input type="text" name="target_url" required value={session.target_url} size="40"/></td>
                </tr>
                <tr>
                    <td><label>"Disable TLS Verify"</label></td>
                    <td><input type="checkbox" name="tls_verify_disabled" value="1" checked={tls_disabled}/></td>
                </tr>
                <tr>
                    <td><label>"Authorization Header"</label></td>
                    <td><input type="text" name="auth_header" value={auth_header_val} placeholder="Bearer sk-..." size="40"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save"/></td>
                </tr>
            </table>
        </form>
        <h2>"Requests"</h2>
        <table>
            <tr><td><a href={requests_href}>"Requests"</a></td></tr>
        </table>
    };

    page_layout(
        &format!("Gateway Proxy - Edit Session {}", session_name),
        body.to_html(),
    )
}
