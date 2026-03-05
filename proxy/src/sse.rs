pub struct SseParser {
    buffer: String,
    current_event_type: String,
    current_data: Vec<String>,
}

impl SseParser {
    pub fn new() -> Self {
        SseParser {
            buffer: String::new(),
            current_event_type: String::new(),
            current_data: Vec::new(),
        }
    }

    /// Feed a chunk of text and return completed `(event_type, data_str)` pairs.
    /// `event_type` is `""` when absent.
    pub fn feed(&mut self, chunk: &str) -> Vec<(String, String)> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].trim_end_matches('\r').to_string();
            self.buffer = self.buffer[pos + 1..].to_string();

            if line.is_empty() {
                if !self.current_data.is_empty() {
                    events.push((
                        self.current_event_type.clone(),
                        self.current_data.join("\n"),
                    ));
                    self.current_data.clear();
                    self.current_event_type.clear();
                }
            } else if let Some(rest) = line.strip_prefix("event:") {
                self.current_event_type =
                    rest.strip_prefix(' ').unwrap_or(rest).to_string();
            } else if let Some(rest) = line.strip_prefix("data:") {
                let data = rest.strip_prefix(' ').unwrap_or(rest);
                self.current_data.push(data.to_string());
            }
        }

        events
    }

    /// Flush any remaining buffered event at end of stream.
    pub fn flush(&mut self) -> Option<(String, String)> {
        if self.current_data.is_empty() {
            None
        } else {
            let event_type = self.current_event_type.clone();
            let data = self.current_data.join("\n");
            self.current_data.clear();
            self.current_event_type.clear();
            Some((event_type, data))
        }
    }
}

/// Re-serialise a parsed event back to SSE wire format.
pub fn serialize_sse_event(event_type: &str, data_str: &str) -> String {
    if event_type.is_empty() {
        format!("data: {}\n\n", data_str)
    } else {
        format!("event: {}\ndata: {}\n\n", event_type, data_str)
    }
}

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
                Ok(parsed) => parsed,
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
        if let Some(delta) = event.get("data").and_then(|data| data.get("delta")) {
            if delta.get("type").and_then(|field| field.as_str()) == Some("text_delta") {
                if let Some(text_content) = delta.get("text").and_then(|field| field.as_str()) {
                    text.push_str(text_content);
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
