use common::models::Session;
use leptos::{either::Either, prelude::*};
use proxy::websearch::PendingToolInfo;
use templates::{html_escape, Breadcrumb, NavLink, Page};

pub fn render_websearch(session: &Session, pending: &[(String, Vec<PendingToolInfo>)]) -> String {
    let session_id = session.id.to_string();
    let ws_enable_action = format!("/_dashboard/sessions/{}/websearch", session_id);
    let ws_disable_action = format!("/_dashboard/sessions/{}/websearch/clear", session_id);
    let wf_enable_action = format!("/_dashboard/sessions/{}/websearch/webfetch", session_id);
    let wf_disable_action = format!(
        "/_dashboard/sessions/{}/websearch/webfetch/clear",
        session_id
    );
    let ws_tool_names_action = format!("/_dashboard/sessions/{}/websearch/tool-names", session_id);
    let wf_tool_names_action = format!(
        "/_dashboard/sessions/{}/websearch/webfetch/tool-names",
        session_id
    );
    let whitelist_save_action = format!("/_dashboard/sessions/{}/websearch/whitelist", session_id);
    let whitelist_clear_action = format!(
        "/_dashboard/sessions/{}/websearch/whitelist/clear",
        session_id
    );

    let whitelist_value = session.websearch_whitelist.clone().unwrap_or_default();
    let has_whitelist = session
        .websearch_whitelist
        .as_ref()
        .is_some_and(|w| !w.trim().is_empty());

    let ws_active = session.websearch_intercept;
    let wf_active = session.webfetch_intercept;
    let either_active = ws_active || wf_active;

    let ws_tool_names_value = session.websearch_tool_names.clone();
    let wf_tool_names_value = session.webfetch_tool_names.clone();

    let mut pending_html = String::new();
    if !pending.is_empty() {
        pending_html.push_str("<table><tr><th>Tool</th><th>Input</th><th></th></tr>");
        for (approval_id, tools) in pending {
            for tool in tools {
                let fail_action = format!(
                    "/_dashboard/sessions/{}/websearch/fail/{}",
                    session_id, approval_id
                );
                let mock_action = format!(
                    "/_dashboard/sessions/{}/websearch/mock/{}",
                    session_id, approval_id
                );
                let accept_action = format!(
                    "/_dashboard/sessions/{}/websearch/accept/{}",
                    session_id, approval_id
                );
                pending_html.push_str(&format!(
                    "<tr><td><code>{}</code></td><td>{}</td><td>\
                     <form method=\"POST\" action=\"{}\">\
                     <button type=\"submit\">Accept</button>\
                     </form> \
                     <form method=\"POST\" action=\"{}\">\
                     <button type=\"submit\">Fail</button>\
                     </form> \
                     <form method=\"POST\" action=\"{}\">\
                     <button type=\"submit\">Mock</button>\
                     </form>\
                     </td></tr>",
                    html_escape(&tool.name),
                    html_escape(&tool.input_summary),
                    html_escape(&accept_action),
                    html_escape(&fail_action),
                    html_escape(&mock_action),
                ));
            }
        }
        pending_html.push_str("</table>");
    }

    let content = view! {
        {if either_active {
            Some(view! { <meta http-equiv="refresh" content="2" /> })
        } else {
            None
        }}

        // ── WebSearch Intercept section ──────────────────────────────
        <h2>"WebSearch Intercept"</h2>
        {if ws_active {
            Either::Left(view! {
                <p>
                    "WebSearch interception is "
                    <strong>"enabled"</strong>
                    ". Tool calls matching the names below will be paused for approval."
                    " "
                    <form method="POST" action={ws_disable_action}>
                        <button type="submit">"Disable"</button>
                    </form>
                </p>
            })
        } else {
            Either::Right(view! {
                <p>
                    "WebSearch interception is disabled."
                    " "
                    <form method="POST" action={ws_enable_action}>
                        <button type="submit">"Enable"</button>
                    </form>
                </p>
            })
        }}
        <h3>"Tool Names"</h3>
        <p>"One tool name per line. Only exact matches are intercepted."</p>
        <form method="POST" action={ws_tool_names_action}>
            <table>
                <tr>
                    <td><label>"Names"</label></td>
                    <td><textarea name="tool_names" rows="4" cols="40">{ws_tool_names_value}</textarea></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save" /></td>
                </tr>
            </table>
        </form>

        // ── WebFetch Intercept section ───────────────────────────────
        <h2>"WebFetch Intercept"</h2>
        {if wf_active {
            Either::Left(view! {
                <p>
                    "WebFetch interception is "
                    <strong>"enabled"</strong>
                    ". Tool calls matching the names below will be paused for approval."
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
        <h3>"Tool Names"</h3>
        <p>"One tool name per line. Only exact matches are intercepted."</p>
        <form method="POST" action={wf_tool_names_action}>
            <table>
                <tr>
                    <td><label>"Names"</label></td>
                    <td><textarea name="tool_names" rows="4" cols="40">{wf_tool_names_value}</textarea></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save" /></td>
                </tr>
            </table>
        </form>

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

        // ── Pending Approvals (shared) ───────────────────────────────
        {if pending.is_empty() {
            Either::Left(if either_active {
                Either::Left(view! {
                    <h2>"Pending Approvals"</h2>
                    <p>"No pending approvals."</p>
                })
            } else {
                Either::Right(())
            })
        } else {
            Either::Right(view! {
                <h2>{format!("Pending Approvals ({})", pending.len())}</h2>
                <div inner_html={pending_html} />
            })
        }}

        <h2>"How It Works"</h2>
        <p>"When enabled, the proxy:"</p>
        <ol>
            <li>"Forwards requests to upstream as normal"</li>
            <li>"Parses the response for tool_use blocks matching the names above"</li>
            <li>"If found: pauses and waits for your approval (Accept, Fail, or Mock)"</li>
            <li>"Accept: fetches the URL for real, converts HTML to text, returns content"</li>
            <li>"Fail: returns an error rejection to the client"</li>
            <li>"Mock: returns a mock result and sends a follow-up request"</li>
            <li>"Times out after 120 seconds (auto-fail)"</li>
        </ol>
    };

    Page {
        title: format!(
            "Gateway Proxy - Session {} - WebSearch Intercept",
            session.name
        ),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Sessions", "/_dashboard/sessions"),
            Breadcrumb::link(
                format!("Session {}", session.name),
                format!("/_dashboard/sessions/{}", session_id),
            ),
            Breadcrumb::current("WebSearch Intercept"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
