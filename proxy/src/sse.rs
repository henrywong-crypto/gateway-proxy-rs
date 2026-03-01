pub fn parse_sse_events(body: &str) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    let mut current_event_type = String::new();
    let mut current_data = String::new();

    for line in body.lines() {
        if let Some(stripped) = line.strip_prefix("event:") {
            current_event_type = stripped.trim().to_string();
        } else if let Some(stripped) = line.strip_prefix("data:") {
            if !current_data.is_empty() {
                current_data.push('\n');
            }
            current_data.push_str(stripped.trim());
        } else if line.trim().is_empty() && !current_data.is_empty() {
            // Empty line = end of event
            let data_value = match serde_json::from_str::<serde_json::Value>(&current_data) {
                Ok(v) => v,
                Err(_) => serde_json::Value::String(current_data.clone()),
            };
            let mut event = serde_json::Map::new();
            if !current_event_type.is_empty() {
                event.insert(
                    "event".to_string(),
                    serde_json::Value::String(current_event_type.clone()),
                );
            }
            event.insert("data".to_string(), data_value);
            events.push(serde_json::Value::Object(event));
            current_data.clear();
            current_event_type.clear();
        }
    }

    // Handle trailing event without final blank line
    if !current_data.is_empty() {
        let data_value = match serde_json::from_str::<serde_json::Value>(&current_data) {
            Ok(v) => v,
            Err(_) => serde_json::Value::String(current_data.clone()),
        };
        let mut event = serde_json::Map::new();
        if !current_event_type.is_empty() {
            event.insert(
                "event".to_string(),
                serde_json::Value::String(current_event_type),
            );
        }
        event.insert("data".to_string(), data_value);
        events.push(serde_json::Value::Object(event));
    }

    events
}

/// Extract all text from `text_delta` events in a parsed SSE event list.
/// Returns the concatenated text content from the response.
pub fn extract_text_from_events(events: &[serde_json::Value]) -> String {
    let mut text = String::new();
    for event in events {
        if let Some(delta) = event.get("data").and_then(|d| d.get("delta")) {
            if delta.get("type").and_then(|v| v.as_str()) == Some("text_delta") {
                if let Some(t) = delta.get("text").and_then(|v| v.as_str()) {
                    text.push_str(t);
                }
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_json_event() {
        let body = "event: message_start\ndata: {\"type\":\"message_start\"}\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event"], "message_start");
        assert!(events[0]["data"].is_object());
        assert_eq!(events[0]["data"]["type"], "message_start");
    }

    #[test]
    fn non_json_data() {
        let body = "event: ping\ndata: just a string\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["data"], "just a string");
    }

    #[test]
    fn multiple_events() {
        let body = "event: a\ndata: {\"x\":1}\n\nevent: b\ndata: {\"x\":2}\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["event"], "a");
        assert_eq!(events[0]["data"]["x"], 1);
        assert_eq!(events[1]["event"], "b");
        assert_eq!(events[1]["data"]["x"], 2);
    }

    #[test]
    fn trailing_event_without_blank_line() {
        let body = "event: last\ndata: {\"done\":true}";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event"], "last");
        assert_eq!(events[0]["data"]["done"], true);
    }

    #[test]
    fn empty_input() {
        let events = parse_sse_events("");
        assert!(events.is_empty());
    }

    #[test]
    fn data_only_no_event_field() {
        let body = "data: hello\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 1);
        // No "event" key should be present
        assert!(events[0].get("event").is_none());
        assert_eq!(events[0]["data"], "hello");
    }
}
