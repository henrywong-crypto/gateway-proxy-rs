use leptos::either::Either;
use leptos::prelude::*;

use crate::pages::page_layout;
use common::models::SystemFilter;

pub fn render_filters_page(filters: &[SystemFilter]) -> String {
    let filters = filters.to_vec();
    let empty = filters.is_empty();
    let form_action = "/_dashboard/filters".to_string();
    let existing_patterns: Vec<String> = filters.iter().map(|f| f.pattern.clone()).collect();
    let suggestions: Vec<&&str> = db::DEFAULT_FILTER_SUGGESTIONS
        .iter()
        .filter(|s| !existing_patterns.contains(&s.to_string()))
        .collect();
    let has_suggestions = !suggestions.is_empty();
    let body = view! {
        <h1>
            <a href="/_dashboard">"Home"</a>
            " / "
            "System Filters"
        </h1>
        <h2>"Navigation"</h2>
        <table>
            <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
        </table>
        <h2>"Add Filter"</h2>
        <form method="POST" action={form_action.clone()}>
            <table>
                <tr>
                    <td><label>"Pattern"</label></td>
                    <td><input type="text" name="pattern" required placeholder="regex pattern, e.g. ^You are Claude" size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Add Filter"/></td>
                </tr>
            </table>
        </form>
        {if has_suggestions {
            Either::Left(view! {
                <h2>"Suggested Defaults"</h2>
                <p>"Click to add a suggested filter pattern:"</p>
                <table>
                    {suggestions.into_iter().map(|s| {
                        let pattern = s.to_string();
                        view! {
                            <tr>
                                <td><code>{pattern.clone()}</code></td>
                                <td>
                                    <form method="POST" action={form_action.clone()} style="display:inline">
                                        <input type="hidden" name="pattern" value={pattern}/>
                                        <button type="submit">"Add"</button>
                                    </form>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        } else {
            Either::Right(view! {})
        }}
        <h2>"Filters"</h2>
        {if empty {
            Either::Left(view! {
                <p>"No filters configured."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"ID"</th>
                        <th>"Pattern"</th>
                        <th>"Created"</th>
                        <th></th>
                    </tr>
                    {filters.into_iter().map(|f| {
                        let edit_action = format!("{}?edit={}", form_action, f.id);
                        let delete_action = format!("{}?delete={}", form_action, f.id);
                        let pattern = f.pattern.clone();
                        view! {
                            <tr>
                                <td>{f.id}</td>
                                <td>
                                    <form method="POST" action={edit_action} style="display:inline">
                                        <input type="text" name="pattern" value={pattern} size="60"/>
                                        <button type="submit">"Save"</button>
                                    </form>
                                </td>
                                <td>{f.created_at.clone().unwrap_or_default()}</td>
                                <td>
                                    <form method="POST" action={delete_action} style="display:inline">
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
    page_layout(
        "Gateway Proxy - System Filters",
        body.to_html(),
    )
}
