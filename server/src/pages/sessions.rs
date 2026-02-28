use common::models::{FilterProfile, Session};
use leptos::{either::Either, prelude::*};
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

pub fn render_sessions_index(sessions: &[Session]) -> String {
    let sessions = sessions.to_vec();
    let empty = sessions.is_empty();
    let total = sessions.len();

    let content = view! {
        <h2>"Sessions"</h2>
        <p>{format!("Total: {}", total)}</p>
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
                        let id_str = s.id.to_string();
                        view! {
                            <tr>
                                <td><a href={href}>{id_str}</a></td>
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

    Page {
        title: "Gateway Proxy - Sessions".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::current("Sessions"),
        ],
        nav_links: vec![
            NavLink::new("New Session", "/_dashboard/sessions/new"),
            NavLink::back(),
        ],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}

pub fn render_new_session(profiles: &[FilterProfile], default_profile_id: &str) -> String {
    let profiles = profiles.to_vec();
    let default_profile_id = default_profile_id.to_string();

    let form = view! {
        <h2>"New Session"</h2>
        <form method="POST" action="/_dashboard/sessions/new">
            <table>
                <tr>
                    <td><label>"Name"</label></td>
                    <td><input type="text" name="name" required/></td>
                </tr>
                <tr>
                    <td><label>"Target URL"</label></td>
                    <td><input type="text" name="target_url" required placeholder="https://api.example.com" size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Filter Profile"</label></td>
                    <td>
                        <select name="profile_id">
                            {profiles.into_iter().map(|p| {
                                let pid = p.id.to_string();
                                let selected = pid == default_profile_id;
                                let label = if p.is_default {
                                    format!("{} (default)", p.name)
                                } else {
                                    p.name.clone()
                                };
                                view! {
                                    <option value={pid} selected={selected}>{label}</option>
                                }
                            }).collect::<Vec<_>>()}
                        </select>
                    </td>
                </tr>
                <tr>
                    <td><label>"Disable TLS Verify"</label></td>
                    <td><input type="checkbox" name="tls_verify_disabled" value="1"/></td>
                </tr>
                <tr>
                    <td><label>"Authorization Header"</label></td>
                    <td><input type="text" name="auth_header" placeholder="Bearer sk-..." size="60"/></td>
                </tr>
                <tr>
                    <td><label>"X-API-Key Header"</label></td>
                    <td><input type="text" name="x_api_key" placeholder="sk-..." size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Create"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: "Gateway Proxy - New Session".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::current("New Session"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content: form,
        subpages: vec![],
    }
    .render()
}

pub fn render_edit_session(session: &Session, port: u16, profiles: &[FilterProfile]) -> String {
    let session = session.clone();
    let session_name = session.name.clone();
    let edit_action = format!("/_dashboard/sessions/{}/edit", session.id);
    let proxy_url = format!("http://localhost:{}/_proxy/{}/", port, session.id);
    let tls_disabled = session.tls_verify_disabled;
    let auth_header_val = session.auth_header.clone().unwrap_or_default();
    let x_api_key_val = session.x_api_key.clone().unwrap_or_default();
    let current_profile_id = session.profile_id.clone().unwrap_or_default();
    let profiles = profiles.to_vec();

    let form = view! {
        <h2>"Edit Session"</h2>
        <form method="POST" action={edit_action}>
            <table>
                <tr>
                    <td><label>"Name"</label></td>
                    <td><input type="text" name="name" required value={session.name}/></td>
                </tr>
                <tr>
                    <td><label>"Target URL"</label></td>
                    <td><input type="text" name="target_url" required value={session.target_url} size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Filter Profile"</label></td>
                    <td>
                        <select name="profile_id">
                            {profiles.into_iter().map(|p| {
                                let pid = p.id.to_string();
                                let selected = pid == current_profile_id;
                                let label = if p.is_default {
                                    format!("{} (default)", p.name)
                                } else {
                                    p.name.clone()
                                };
                                view! {
                                    <option value={pid} selected={selected}>{label}</option>
                                }
                            }).collect::<Vec<_>>()}
                        </select>
                    </td>
                </tr>
                <tr>
                    <td><label>"Disable TLS Verify"</label></td>
                    <td><input type="checkbox" name="tls_verify_disabled" value="1" checked={tls_disabled}/></td>
                </tr>
                <tr>
                    <td><label>"Authorization Header"</label></td>
                    <td><input type="text" name="auth_header" value={auth_header_val} size="60"/></td>
                </tr>
                <tr>
                    <td><label>"X-API-Key Header"</label></td>
                    <td><input type="text" name="x_api_key" value={x_api_key_val} size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: format!("Gateway Proxy - Edit Session {}", session_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::link(
                format!("Session {}", session_name),
                format!("/_dashboard/sessions/{}", session.id),
            ),
            Breadcrumb::current("Edit"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![InfoRow::new("Proxy URL", &proxy_url)],
        content: form,
        subpages: vec![Subpage::new(
            "Requests",
            format!("/_dashboard/sessions/{}/requests", session.id),
            session.request_count,
        )],
    }
    .render()
}
