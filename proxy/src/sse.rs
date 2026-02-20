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
