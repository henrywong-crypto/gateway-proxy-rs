use serde_json::Value;

pub fn truncate_strings(val: &Value, max_len: usize) -> Value {
    match val {
        Value::String(s) => {
            if s.len() > max_len {
                let truncated: String = s.chars().take(max_len).collect();
                Value::String(format!("{}...", truncated))
            } else {
                val.clone()
            }
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| truncate_strings(v, max_len)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), truncate_strings(v, max_len)))
                .collect(),
        ),
        _ => val.clone(),
    }
}
