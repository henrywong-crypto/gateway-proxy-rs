use std::collections::HashSet;

use leptos::{either::Either, prelude::*};

use crate::collapsible_block;

pub fn render_messages(json_str: &str, order: &str, keep_tool_pairs: i64) -> AnyView {
    let Ok(mut msgs) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
        let s = json_str.to_string();
        return view! { <pre>{s}</pre> }.into_any();
    };

    // Collect tool_use IDs to determine which are filtered
    let filtered_ids: HashSet<String> = if keep_tool_pairs > 0 {
        let mut all_ids: Vec<String> = Vec::new();
        for msg in &msgs {
            if let Some(blocks) = msg.get("content").and_then(|c| c.as_array()) {
                for block in blocks {
                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        if let Some(id) = block.get("id").and_then(|i| i.as_str()) {
                            all_ids.push(id.to_string());
                        }
                    }
                }
            }
        }
        let keep = keep_tool_pairs as usize;
        if all_ids.len() > keep {
            all_ids[..all_ids.len() - keep].iter().cloned().collect()
        } else {
            HashSet::new()
        }
    } else {
        HashSet::new()
    };

    if order == "desc" {
        msgs.reverse();
    }

    let rows: Vec<AnyView> = msgs
        .iter()
        .flat_map(|msg| {
            let role = msg
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown");

            let content = &msg["content"];
            if let Some(s) = content.as_str() {
                let role = role.to_string();
                let cb = collapsible_block(s, "");
                vec![view! {
                    <tr><td>{role}</td><td>"text"</td><td>{cb}</td></tr>
                }
                .into_any()]
            } else if let Some(blocks) = content.as_array() {
                blocks
                    .iter()
                    .enumerate()
                    .filter_map(|(i, block)| {
                        let btype = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        let role_cell = if i == 0 {
                            role.to_string()
                        } else {
                            String::new()
                        };

                        // Determine if this block is filtered
                        let is_filtered = match btype {
                            "tool_use" => {
                                let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                filtered_ids.contains(id)
                            }
                            "tool_result" => {
                                let id = block
                                    .get("tool_use_id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("");
                                filtered_ids.contains(id)
                            }
                            _ => false,
                        };
                        let row_class = if is_filtered { "filtered-row" } else { "" };
                        let filtered_badge = if is_filtered {
                            Either::Left(view! {
                                " " <span class="filtered-badge">"[FILTERED]"</span>
                            })
                        } else {
                            Either::Right(())
                        };

                        match btype {
                            "text" => {
                                let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                let cache_info = format_cache_control_label(block);
                                let type_label = format!("text{}", cache_info);
                                let cb = collapsible_block(text, "");
                                Some(
                                    view! {
                                        <tr>
                                            <td>{role_cell}</td>
                                            <td>{type_label}</td>
                                            <td>{cb}</td>
                                        </tr>
                                    }
                                    .into_any(),
                                )
                            }
                            "thinking" => {
                                let text =
                                    block.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
                                let cache_info = format_cache_control_label(block);
                                let type_label = format!("thinking{}", cache_info);
                                let cb = collapsible_block(text, "");
                                Some(
                                    view! {
                                        <tr>
                                            <td>{role_cell}</td>
                                            <td>{type_label}</td>
                                            <td>{cb}</td>
                                        </tr>
                                    }
                                    .into_any(),
                                )
                            }
                            "tool_use" => {
                                let name = block
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let id = block
                                    .get("id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let cache_info = format_cache_control_label(block);
                                let type_label = format!("tool_use{}", cache_info);

                                let params_rows: Vec<AnyView> = block
                                    .get("input")
                                    .and_then(|i| i.as_object())
                                    .map(|input| {
                                        input
                                            .iter()
                                            .map(|(k, v)| {
                                                let val = if v.is_string() {
                                                    v.as_str().unwrap_or("").to_string()
                                                } else {
                                                    serde_json::to_string(v).unwrap_or_default()
                                                };
                                                let k = k.clone();
                                                let cb = collapsible_block(&val, "");
                                                view! {
                                                    <tr><td>{k}</td><td>{cb}</td></tr>
                                                }
                                                .into_any()
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                let params_view = if !params_rows.is_empty() {
                                    Either::Left(view! {
                                        <table>
                                            <tr><th>"Param"</th><th>"Value"</th></tr>
                                            {params_rows}
                                        </table>
                                    })
                                } else {
                                    Either::Right(())
                                };

                                Some(
                                    view! {
                                        <tr class={row_class}>
                                            <td>{role_cell}</td>
                                            <td>{type_label}{filtered_badge}": "{name}" "{id}</td>
                                            <td>{params_view}</td>
                                        </tr>
                                    }
                                    .into_any(),
                                )
                            }
                            "tool_result" => {
                                let tool_use_id = block
                                    .get("tool_use_id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let cache_info = format_cache_control_label(block);
                                let type_label = format!("tool_result{}", cache_info);
                                let result_text = if let Some(s) =
                                    block.get("content").and_then(|c| c.as_str())
                                {
                                    s.to_string()
                                } else if let Some(arr) =
                                    block.get("content").and_then(|c| c.as_array())
                                {
                                    arr.iter()
                                        .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                } else {
                                    String::new()
                                };
                                let cb = collapsible_block(&result_text, "");
                                Some(
                                    view! {
                                        <tr class={row_class}>
                                            <td>{role_cell}</td>
                                            <td>{type_label}{filtered_badge}" "{tool_use_id}</td>
                                            <td>{cb}</td>
                                        </tr>
                                    }
                                    .into_any(),
                                )
                            }
                            _ => None,
                        }
                    })
                    .collect()
            } else {
                let role = role.to_string();
                vec![view! {
                    <tr><td>{role}</td><td></td><td></td></tr>
                }
                .into_any()]
            }
        })
        .collect();

    view! {
        <table>
            <tr><th>"Role"</th><th>"Type"</th><th>"Content"</th></tr>
            {rows}
        </table>
    }
    .into_any()
}

fn format_cache_control_label(block: &serde_json::Value) -> String {
    block
        .get("cache_control")
        .and_then(|c| c.get("type"))
        .and_then(|t| t.as_str())
        .map(|t| format!(" (cache: {})", t))
        .unwrap_or_default()
}
