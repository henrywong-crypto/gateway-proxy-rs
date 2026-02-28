use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

/// Returns true if text matches the pattern (tried as regex first, then substring).
fn pattern_matches(text: &str, pattern: &str) -> bool {
    match Regex::new(pattern) {
        Ok(re) => re.is_match(text),
        Err(_) => text.contains(pattern),
    }
}

/// Apply all configured filters to a JSON request body in place.
///
/// - **system_filters**: regex/substring patterns — matching system text blocks are removed.
/// - **tool_filters**: tool names — matching tool entries are removed from the `tools` array.
/// - **keep_tool_pairs**: if > 0, only the last N tool_use/tool_result pairs are kept in messages;
///   older pairs are removed. Messages whose content becomes empty are removed entirely.
pub fn apply_filters(
    body: &mut Value,
    system_filters: &[String],
    tool_filters: &[String],
    keep_tool_pairs: i64,
) {
    apply_system_filters(body, system_filters);
    apply_tool_filters(body, tool_filters);
    if keep_tool_pairs > 0 {
        apply_message_filters(body, keep_tool_pairs as usize);
    }
}

fn apply_system_filters(body: &mut Value, filters: &[String]) {
    if filters.is_empty() {
        return;
    }

    let system = match body.get_mut("system") {
        Some(v) => v,
        None => return,
    };

    if let Some(s) = system.as_str().map(|s| s.to_string()) {
        if filters.iter().any(|f| pattern_matches(&s, f)) {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("system");
            }
        }
        return;
    }

    if let Some(arr) = system.as_array_mut() {
        arr.retain(|block| {
            let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
            !filters.iter().any(|f| pattern_matches(text, f))
        });
        if arr.is_empty() {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("system");
            }
        }
    }
}

fn apply_tool_filters(body: &mut Value, filters: &[String]) {
    if filters.is_empty() {
        return;
    }

    let tools = match body.get_mut("tools") {
        Some(v) => v,
        None => return,
    };

    if let Some(arr) = tools.as_array_mut() {
        arr.retain(|tool| {
            let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("");
            !filters.iter().any(|f| f == name)
        });
        if arr.is_empty() {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("tools");
            }
        }
    }
}

fn apply_message_filters(body: &mut Value, keep: usize) {
    let messages = match body.get_mut("messages") {
        Some(Value::Array(arr)) => arr,
        _ => return,
    };

    // 1. Collect all tool_use IDs in chronological order
    let mut all_tool_ids: Vec<String> = Vec::new();
    for msg in messages.iter() {
        if let Some(blocks) = msg.get("content").and_then(|c| c.as_array()) {
            for block in blocks {
                if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if let Some(id) = block.get("id").and_then(|i| i.as_str()) {
                        all_tool_ids.push(id.to_string());
                    }
                }
            }
        }
    }

    if all_tool_ids.len() <= keep {
        return;
    }

    // 2. IDs to remove: all except the last `keep`
    let remove_count = all_tool_ids.len() - keep;
    let ids_to_remove: HashSet<&str> = all_tool_ids[..remove_count]
        .iter()
        .map(|s| s.as_str())
        .collect();

    // 3. Filter content blocks and remove empty messages
    messages.retain_mut(|msg| {
        let content = match msg.get_mut("content") {
            Some(Value::Array(arr)) => arr,
            _ => return true,
        };

        content.retain(|block| {
            let btype = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match btype {
                "tool_use" => {
                    let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    !ids_to_remove.contains(id)
                }
                "tool_result" => {
                    let id = block
                        .get("tool_use_id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("");
                    !ids_to_remove.contains(id)
                }
                _ => true,
            }
        });

        !content.is_empty()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn system_string_removed_when_matched() {
        let mut body = json!({
            "system": "You are Claude Code, Anthropic's official CLI for Claude.",
            "messages": []
        });
        apply_filters(&mut body, &["^You are Claude Code".to_string()], &[], 0);
        assert!(body.get("system").is_none());
    }

    #[test]
    fn system_string_kept_when_no_match() {
        let mut body = json!({
            "system": "You are a helpful assistant.",
            "messages": []
        });
        apply_filters(&mut body, &["^You are Claude Code".to_string()], &[], 0);
        assert!(body.get("system").is_some());
    }

    #[test]
    fn system_array_partial_removal() {
        let mut body = json!({
            "system": [
                {"type": "text", "text": "keep this"},
                {"type": "text", "text": "remove this secret"}
            ],
            "messages": []
        });
        apply_filters(&mut body, &["secret".to_string()], &[], 0);
        let arr = body["system"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["text"].as_str().unwrap(), "keep this");
    }

    #[test]
    fn system_array_fully_removed() {
        let mut body = json!({
            "system": [
                {"type": "text", "text": "secret A"},
                {"type": "text", "text": "secret B"}
            ],
            "messages": []
        });
        apply_filters(&mut body, &["secret".to_string()], &[], 0);
        assert!(body.get("system").is_none());
    }

    #[test]
    fn tool_filter_removes_matching() {
        let mut body = json!({
            "tools": [
                {"name": "WebSearch"},
                {"name": "Calculator"},
                {"name": "Bash"}
            ],
            "messages": []
        });
        apply_filters(
            &mut body,
            &[],
            &["WebSearch".to_string(), "Bash".to_string()],
            0,
        );
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"].as_str().unwrap(), "Calculator");
    }

    #[test]
    fn tool_filter_removes_field_when_empty() {
        let mut body = json!({
            "tools": [{"name": "WebSearch"}],
            "messages": []
        });
        apply_filters(&mut body, &[], &["WebSearch".to_string()], 0);
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn message_filter_keeps_last_n_pairs() {
        let mut body = json!({
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "hi"}]},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "tu1", "name": "A", "input": {}},
                    {"type": "tool_use", "id": "tu2", "name": "B", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu1", "content": "r1"},
                    {"type": "tool_result", "tool_use_id": "tu2", "content": "r2"}
                ]},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "tu3", "name": "C", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu3", "content": "r3"}
                ]},
                {"role": "assistant", "content": [{"type": "text", "text": "done"}]}
            ]
        });
        apply_filters(&mut body, &[], &[], 2);
        let msgs = body["messages"].as_array().unwrap();

        // tu1 should be removed, tu2 and tu3 kept
        // Check that tu1 is gone from assistant message
        let assistant1 = &msgs[1]["content"].as_array().unwrap();
        assert_eq!(assistant1.len(), 1);
        assert_eq!(assistant1[0]["id"].as_str().unwrap(), "tu2");

        // Check that tu1 result is gone
        let user1 = &msgs[2]["content"].as_array().unwrap();
        assert_eq!(user1.len(), 1);
        assert_eq!(user1[0]["tool_use_id"].as_str().unwrap(), "tu2");
    }

    #[test]
    fn message_filter_removes_empty_messages() {
        let mut body = json!({
            "messages": [
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "tu1", "name": "A", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu1", "content": "r1"}
                ]},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "tu2", "name": "B", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu2", "content": "r2"}
                ]}
            ]
        });
        apply_filters(&mut body, &[], &[], 1);
        let msgs = body["messages"].as_array().unwrap();
        // tu1 pair removed entirely (messages become empty → removed)
        assert_eq!(msgs.len(), 2);
        assert_eq!(
            msgs[0]["content"].as_array().unwrap()[0]["id"]
                .as_str()
                .unwrap(),
            "tu2"
        );
    }

    #[test]
    fn message_filter_no_op_when_fewer_than_keep() {
        let mut body = json!({
            "messages": [
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "tu1", "name": "A", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu1", "content": "r1"}
                ]}
            ]
        });
        let original = body.clone();
        apply_filters(&mut body, &[], &[], 5);
        assert_eq!(body, original);
    }

    #[test]
    fn all_filters_combined() {
        let mut body = json!({
            "system": "secret system prompt",
            "tools": [{"name": "WebSearch"}, {"name": "Calc"}],
            "messages": [
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "tu1", "name": "A", "input": {}},
                    {"type": "tool_use", "id": "tu2", "name": "B", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu1", "content": "r1"},
                    {"type": "tool_result", "tool_use_id": "tu2", "content": "r2"}
                ]}
            ]
        });
        apply_filters(
            &mut body,
            &["secret".to_string()],
            &["WebSearch".to_string()],
            1,
        );
        assert!(body.get("system").is_none());
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"].as_str().unwrap(), "Calc");
        let msgs = body["messages"].as_array().unwrap();
        // tu1 removed, only tu2 left
        let assistant = msgs[0]["content"].as_array().unwrap();
        assert_eq!(assistant.len(), 1);
        assert_eq!(assistant[0]["id"].as_str().unwrap(), "tu2");
    }
}
