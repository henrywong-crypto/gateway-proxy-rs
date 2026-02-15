use leptos::either::Either;
use leptos::prelude::*;

use crate::models::SessionWithCount;
use crate::pages::page_layout;

pub fn render_home(sessions: &[SessionWithCount]) -> String {
    let sessions = sessions.to_vec();
    let empty = sessions.is_empty();
    let body = view! {
        <h1>"Gateway Proxy"</h1>
        <div class="card" style="margin-bottom: 16px;">
            <h3>"Create New Session"</h3>
            <form method="POST" action="/__proxy__/sessions">
                <div class="form-row">
                    <label>"Name:"</label>
                    <input type="text" name="name" required placeholder="my-session"/>
                </div>
                <div class="form-row">
                    <label>"Target URL:"</label>
                    <input type="text" name="target_url" required placeholder="https://api.example.com" size="40"/>
                </div>
                <div class="form-row">
                    <input type="submit" value="Create Session"/>
                </div>
            </form>
        </div>
        {if empty {
            Either::Left(view! {
                <p style="color:#888">"No sessions yet."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"Name"</th>
                        <th>"Target URL"</th>
                        <th>"Requests"</th>
                        <th>"Created"</th>
                    </tr>
                    {sessions.into_iter().map(|s| {
                        let href = format!("/__proxy__/s/{}", s.id);
                        view! {
                            <tr>
                                <td><a href={href}>{s.name}</a></td>
                                <td>{s.target_url}</td>
                                <td>{s.request_count}</td>
                                <td>{s.created_at.unwrap_or_default()}</td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        }}
    };
    page_layout("Gateway Proxy", body.to_html())
}
