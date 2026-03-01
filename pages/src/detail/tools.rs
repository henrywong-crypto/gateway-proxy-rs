use leptos::{either::Either, prelude::*};

use crate::collapsible_block;

pub fn render_tools(json_str: &str, filters: &[String]) -> AnyView {
    let Ok(tools) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
        let s = json_str.to_string();
        return view! { <pre>{s}</pre> }.into_any();
    };

    let rows: Vec<AnyView> = tools
        .iter()
        .map(|tool| {
            let name = tool
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("(unnamed)");
            let desc = tool
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            let filtered = filters.iter().any(|f| f == name);
            let row_class = if filtered { "filtered-row" } else { "" };

            let params_rows: Vec<AnyView> = tool
                .get("input_schema")
                .and_then(|s| s.as_object())
                .and_then(|schema| {
                    let props = schema.get("properties")?.as_object()?;
                    let required: Vec<&str> = schema
                        .get("required")
                        .and_then(|r| r.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();

                    Some(
                        props
                            .iter()
                            .map(|(k, v)| {
                                let ptype = v
                                    .get("type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("object")
                                    .to_string();
                                let pdesc = v
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let req = if required.contains(&k.as_str()) {
                                    "yes"
                                } else {
                                    "no"
                                };
                                let k = k.clone();
                                let req = req.to_string();
                                view! {
                                    <tr><td>{k}</td><td>{ptype}</td><td>{req}</td><td>{pdesc}</td></tr>
                                }
                                .into_any()
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default();

            let params_view = if !params_rows.is_empty() {
                Either::Left(view! {
                    <table>
                        <tr><th>"Name"</th><th>"Type"</th><th>"Req"</th><th>"Description"</th></tr>
                        {params_rows}
                    </table>
                })
            } else {
                Either::Right(())
            };

            let name_str = name.to_string();
            let filtered_badge = if filtered {
                Either::Left(view! { " " <span class="filtered-badge">"[FILTERED]"</span> })
            } else {
                Either::Right(())
            };
            let desc_cb = collapsible_block(desc, "");
            view! {
                <tr class={row_class}>
                    <td>{name_str}{filtered_badge}</td>
                    <td>{desc_cb}</td>
                    <td>{params_view}</td>
                </tr>
            }
            .into_any()
        })
        .collect();

    view! {
        <table>
            <tr><th>"Name"</th><th>"Description"</th><th>"Parameters"</th></tr>
            {rows}
        </table>
    }
    .into_any()
}
