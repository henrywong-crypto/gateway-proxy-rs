use leptos::either::Either;
use leptos::prelude::*;

use crate::models::SessionWithCount;
use crate::pages::page_layout;

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
            <tr><td><a href="/_dashboard">"Back"</a></td></tr>
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
                                <td><a href={href}>{s.name}</a></td>
                                <td>{s.target_url}</td>
                                <td>{s.request_count}</td>
                                <td>{s.created_at.unwrap_or_default()}</td>
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
            <tr><td><a href="/_dashboard/sessions">"Back"</a></td></tr>
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
                    <td></td>
                    <td><input type="submit" value="Create"/></td>
                </tr>
            </table>
        </form>
    };
    page_layout("Gateway Proxy - New Session", body.to_html())
}
