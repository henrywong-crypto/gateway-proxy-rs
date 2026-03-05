use common::models::{FilterProfile, Session};
use leptos::{either::Either, prelude::*};
use templates::{pagination_nav, Breadcrumb, InfoRow, NavLink, Page, Pagination, Subpage};

pub fn render_sessions_view(sessions: &[Session], pagination: &Pagination) -> String {
    let sessions = sessions.to_vec();
    let empty = sessions.is_empty();
    let total = pagination.total_items;

    let nav_top = pagination_nav(pagination);
    let nav_bottom = pagination_nav(pagination);

    let content = view! {
        <h2>"Sessions"</h2>
        <p>{format!("Total: {}", total)}</p>
        {nav_top}
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
                    {sessions.into_iter().map(|session| {
                        let href = format!("/_dashboard/sessions/{}", session.id);
                        let clear_action = format!("/_dashboard/sessions/{}/clear", session.id);
                        let delete_action = format!("/_dashboard/sessions/{}/delete", session.id);
                        let id_str = session.id.to_string();
                        view! {
                            <tr>
                                <td><a href={href}>{id_str}</a></td>
                                <td>{session.name}</td>
                                <td>{session.target_url}</td>
                                <td>{session.request_count}</td>
                                <td>{session.created_at.clone()}</td>
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
        {nav_bottom}
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

pub fn render_new_session_form(profiles: &[FilterProfile], default_profile_id: &str) -> String {
    let profiles = profiles.to_vec();
    let default_profile_id = default_profile_id.to_string();

    let form = view! {
        <h2>"New Session"</h2>
        <form method="POST" action="/_dashboard/sessions/new">
            <table>
                <tr>
                    <td><label>"Name"</label></td>
                    <td><input type="text" name="name" required size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Target URL"</label></td>
                    <td><input type="text" name="target_url" required placeholder="https://api.example.com" size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Filter Profile"</label></td>
                    <td>
                        <select name="profile_id">
                            {profiles.into_iter().map(|profile| {
                                let profile_id = profile.id.to_string();
                                let selected = profile_id == default_profile_id;
                                let label = if profile.is_default {
                                    format!("{} (default)", profile.name)
                                } else {
                                    profile.name.clone()
                                };
                                view! {
                                    <option value={profile_id} selected={selected}>{label}</option>
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

pub fn render_edit_session_form(
    session: &Session,
    port: u16,
    profiles: &[FilterProfile],
) -> String {
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
                    <td><input type="text" name="name" required value={session.name} size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Target URL"</label></td>
                    <td><input type="text" name="target_url" required value={session.target_url} size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Filter Profile"</label></td>
                    <td>
                        <select name="profile_id">
                            {profiles.into_iter().map(|profile| {
                                let profile_id = profile.id.to_string();
                                let selected = profile_id == current_profile_id;
                                let label = if profile.is_default {
                                    format!("{} (default)", profile.name)
                                } else {
                                    profile.name.clone()
                                };
                                view! {
                                    <option value={profile_id} selected={selected}>{label}</option>
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
