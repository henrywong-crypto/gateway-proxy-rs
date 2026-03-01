use common::models::{PendingToolInfo, Session};
use leptos::{either::Either, prelude::*};
use templates::{Breadcrumb, NavLink, Page};

pub fn render_webfetch_view(session: &Session) -> String {
    let session_id = session.id.to_string();
    let wf_enable_action = format!("/_dashboard/sessions/{}/intercept/webfetch", session_id);
    let wf_disable_action = format!(
        "/_dashboard/sessions/{}/intercept/webfetch/clear",
        session_id
    );
    let whitelist_save_action = format!(
        "/_dashboard/sessions/{}/intercept/webfetch/whitelist",
        session_id
    );
    let whitelist_clear_action = format!(
        "/_dashboard/sessions/{}/intercept/webfetch/whitelist/clear",
        session_id
    );

    let wf_active = session.webfetch_intercept;
    let whitelist_value = session.webfetch_whitelist.clone().unwrap_or_default();
    let has_whitelist = session
        .webfetch_whitelist
        .as_ref()
        .is_some_and(|w| !w.trim().is_empty());

    let content = view! {
        <h2>"WebFetch Intercept"</h2>
        {if wf_active {
            Either::Left(view! {
                <p>
                    "WebFetch interception is "
                    <strong>"enabled"</strong>
                    ". Matching tool calls will be paused for approval."
                    " "
                    <form method="POST" action={wf_disable_action}>
                        <button type="submit">"Disable"</button>
                    </form>
                </p>
            })
        } else {
            Either::Right(view! {
                <p>
                    "WebFetch interception is disabled."
                    " "
                    <form method="POST" action={wf_enable_action}>
                        <button type="submit">"Enable"</button>
                    </form>
                </p>
            })
        }}

        <h3>"Domain Whitelist"</h3>
        <p>"WebFetch calls to whitelisted domains are auto-accepted without manual approval. One domain per line. A domain like " <code>"github.com"</code> " matches " <code>"github.com"</code> " and any subdomain (e.g. " <code>"api.github.com"</code> ")."</p>
        <form method="POST" action={whitelist_save_action}>
            <table>
                <tr>
                    <td><label>"Domains"</label></td>
                    <td><textarea name="whitelist" rows="6" cols="60">{whitelist_value.clone()}</textarea></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save" /></td>
                </tr>
            </table>
        </form>
        {if has_whitelist {
            Either::Left(view! {
                <form method="POST" action={whitelist_clear_action}>
                    <button type="submit">"Clear Whitelist"</button>
                </form>
            })
        } else {
            Either::Right(())
        }}
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - WebFetch Intercept",
            session.name
        ),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::link(
                format!("Session {}", session.name),
                format!("/_dashboard/sessions/{}", session_id),
            ),
            Breadcrumb::link(
                "Intercept",
                format!("/_dashboard/sessions/{}/intercept", session_id),
            ),
            Breadcrumb::current("WebFetch Intercept"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}

pub fn render_approvals_view(
    session: &Session,
    pending: &[(String, Vec<PendingToolInfo>)],
) -> String {
    let session_id = session.id.to_string();
    let either_active = session.webfetch_intercept;

    let pending_rows: Vec<_> = pending
        .iter()
        .flat_map(|(approval_id, tools)| {
            let session_id = session_id.clone();
            tools.iter().map(move |tool| {
                let fail_action = format!(
                    "/_dashboard/sessions/{}/intercept/approvals/fail/{}",
                    session_id, approval_id
                );
                let mock_action = format!(
                    "/_dashboard/sessions/{}/intercept/approvals/mock/{}",
                    session_id, approval_id
                );
                let accept_action = format!(
                    "/_dashboard/sessions/{}/intercept/approvals/accept/{}",
                    session_id, approval_id
                );
                let name = tool.name.clone();
                let input_summary = tool.input_summary.clone();
                view! {
                    <tr>
                        <td><code>{name}</code></td>
                        <td>{input_summary}</td>
                        <td>
                            <form method="POST" action={accept_action}>
                                <button type="submit">"Accept"</button>
                            </form>
                            " "
                            <form method="POST" action={fail_action}>
                                <button type="submit">"Fail"</button>
                            </form>
                            " "
                            <form method="POST" action={mock_action}>
                                <button type="submit">"Mock"</button>
                            </form>
                        </td>
                    </tr>
                }
            })
        })
        .collect();

    let content = view! {
        {if either_active {
            Some(view! { <meta http-equiv="refresh" content="2" /> })
        } else {
            None
        }}

        <h2>"Pending Approvals"</h2>
        {if pending.is_empty() {
            Either::Left(view! {
                <p>"No pending approvals."</p>
            })
        } else {
            Either::Right(view! {
                <p>{format!("{} pending", pending.len())}</p>
                <table>
                    <tr><th>"Tool"</th><th>"Input"</th><th></th></tr>
                    {pending_rows}
                </table>
            })
        }}
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - Pending Approvals",
            session.name
        ),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::link(
                format!("Session {}", session.name),
                format!("/_dashboard/sessions/{}", session_id),
            ),
            Breadcrumb::link(
                "Intercept",
                format!("/_dashboard/sessions/{}/intercept", session_id),
            ),
            Breadcrumb::current("Pending Approvals"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
