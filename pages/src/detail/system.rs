use leptos::prelude::*;
use regex::Regex;

use crate::collapsible_block;

fn find_matched_filter<'a>(text: &str, filters: &'a [String]) -> Option<&'a str> {
    filters.iter().find_map(|filter| {
        let matched = match Regex::new(filter) {
            Ok(re) => re.is_match(text),
            Err(_) => text.contains(filter.as_str()),
        };
        if matched {
            Some(filter.as_str())
        } else {
            None
        }
    })
}

fn render_filtered_content(text: &str, filter_pattern: Option<&str>) -> AnyView {
    let cb = collapsible_block(text, "");
    if filter_pattern.is_some() {
        view! {
            <span class="filtered-badge">"[FILTERED]"</span>
            " "
            {cb}
        }
        .into_any()
    } else {
        cb
    }
}

pub fn render_system(json_str: &str, filters: &[String]) -> AnyView {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) else {
        let string = json_str.to_string();
        return view! { <pre>{string}</pre> }.into_any();
    };

    if let Some(string) = parsed.as_str() {
        let filter_match = find_matched_filter(string, filters);
        let row_class = if filter_match.is_some() {
            "filtered-row"
        } else {
            ""
        };
        let content = render_filtered_content(string, filter_match);
        return view! {
            <table>
                <tr><th>"Type"</th><th>"Content"</th></tr>
                <tr class={row_class}><td>"text"</td><td>{content}</td></tr>
            </table>
        }
        .into_any();
    }

    if let Some(array) = parsed.as_array() {
        let rows: Vec<AnyView> = array
            .iter()
            .map(|block| {
                let block_type = block.get("type").and_then(|field| field.as_str()).unwrap_or("text");
                let text = block.get("text").and_then(|field| field.as_str()).unwrap_or("");
                let fallback;
                let text = if text.is_empty() {
                    fallback = serde_json::to_string_pretty(block).unwrap_or_default();
                    &fallback
                } else {
                    text
                };
                let filter_match = find_matched_filter(text, filters);
                let row_class = if filter_match.is_some() {
                    "filtered-row"
                } else {
                    ""
                };
                let cache_info = block
                    .get("cache_control")
                    .and_then(|field| field.get("type"))
                    .and_then(|field| field.as_str())
                    .map(|cache_type| format!(" (cache: {})", cache_type))
                    .unwrap_or_default();
                let type_label = format!("{}{}", block_type, cache_info);
                let content = render_filtered_content(text, filter_match);
                view! {
                    <tr class={row_class}><td>{type_label}</td><td>{content}</td></tr>
                }
                .into_any()
            })
            .collect();

        return view! {
            <table>
                <tr><th>"Type"</th><th>"Content"</th></tr>
                {rows}
            </table>
        }
        .into_any();
    }

    let string = serde_json::to_string_pretty(&parsed).unwrap_or_default();
    view! { <pre>{string}</pre> }.into_any()
}
