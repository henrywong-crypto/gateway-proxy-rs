use leptos::{either::Either, prelude::*};

use crate::collapsible_block;

pub fn render_tools(json_str: &str, filters: &[String]) -> AnyView {
    let Ok(tools) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
        let string = json_str.to_string();
        return view! { <pre>{string}</pre> }.into_any();
    };

    let rows: Vec<AnyView> = tools
        .iter()
        .map(|tool| {
            let name = tool
                .get("name")
                .and_then(|field| field.as_str())
                .unwrap_or("(unnamed)");
            let desc = tool
                .get("description")
                .and_then(|field| field.as_str())
                .unwrap_or("");

            let filtered = filters.iter().any(|filter_name| filter_name == name);
            let row_class = if filtered { "filtered-row" } else { "" };

            let params_rows: Vec<AnyView> = tool
                .get("input_schema")
                .and_then(|field| field.as_object())
                .and_then(|schema| {
                    let props = schema.get("properties")?.as_object()?;
                    let required: Vec<&str> = schema
                        .get("required")
                        .and_then(|field| field.as_array())
                        .map(|arr| arr.iter().filter_map(|field| field.as_str()).collect())
                        .unwrap_or_default();

                    Some(
                        props
                            .iter()
                            .map(|(key, value)| {
                                let param_type = value
                                    .get("type")
                                    .and_then(|field| field.as_str())
                                    .unwrap_or("object")
                                    .to_string();
                                let param_description = value
                                    .get("description")
                                    .and_then(|field| field.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let req = if required.contains(&key.as_str()) {
                                    "yes"
                                } else {
                                    "no"
                                };
                                let key = key.clone();
                                let req = req.to_string();
                                view! {
                                    <tr><td>{key}</td><td>{param_type}</td><td>{req}</td><td>{param_description}</td></tr>
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
