use serde_json::Value;

pub fn truncate_strings(value: &Value, max_len: usize) -> Value {
    match value {
        Value::String(string) => {
            if string.len() > max_len {
                let truncated: String = string.chars().take(max_len).collect();
                Value::String(format!("{}...", truncated))
            } else {
                value.clone()
            }
        }
        Value::Array(array) => {
            Value::Array(array.iter().map(|element| truncate_strings(element, max_len)).collect())
        }
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, child)| (key.clone(), truncate_strings(child, max_len)))
                .collect(),
        ),
        _ => value.clone(),
    }
}
