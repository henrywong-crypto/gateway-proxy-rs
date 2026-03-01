/// A known injectable Anthropic error type.
pub struct ErrorType {
    /// The key stored in the DB (e.g. "rate_limit_error").
    pub key: &'static str,
    /// Human-readable label for the UI (e.g. "Rate Limit Error (429)").
    pub label: &'static str,
    /// HTTP status code for this error type.
    pub status: u16,
    /// The JSON error payload returned to the client.
    pub data_json: &'static str,
}

/// All Anthropic error types that can be injected.
pub const ERROR_TYPES: &[ErrorType] = &[
    ErrorType {
        key: "invalid_request_error",
        label: "Context Window Exceeded (400)",
        status: 400,
        data_json: r#"{"type":"error","error":{"type":"invalid_request_error","message":"prompt is too long: 201234 tokens > 200000 maximum"}}"#,
    },
    ErrorType {
        key: "permission_error",
        label: "Permission Error (403)",
        status: 403,
        data_json: r#"{"type":"error","error":{"type":"permission_error","message":"Your API key does not have permission to use the specified resource."}}"#,
    },
    ErrorType {
        key: "not_found_error",
        label: "Not Found (404)",
        status: 404,
        data_json: r#"{"type":"error","error":{"type":"not_found_error","message":"The requested resource could not be found."}}"#,
    },
    ErrorType {
        key: "request_too_large",
        label: "Request Too Large (413)",
        status: 413,
        data_json: r#"{"type":"error","error":{"type":"request_too_large","message":"Request exceeds the maximum allowed number of bytes."}}"#,
    },
];

/// Look up a known error type by its key, or `None` if unknown.
pub fn find_by_key(key: &str) -> Option<&'static ErrorType> {
    ERROR_TYPES.iter().find(|e| e.key == key)
}
