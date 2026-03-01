use handlebars::Handlebars;
use serde_json::Value;

use super::extract::ToolUse;

/// Render a Handlebars template with the given data, falling back to the raw
/// template string on error.
pub(super) fn render_template(template: &str, data: &Value) -> String {
    let mut hbs = Handlebars::new();
    hbs.set_strict_mode(false);
    hbs.register_escape_fn(handlebars::no_escape);
    match hbs.render_template(template, data) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Handlebars template render error: {}", e);
            template.to_string()
        }
    }
}

/// Generate a mock tool_result for a given tool_use.
pub(super) fn build_mock_result(tool_use: &ToolUse, webfetch_prompt: &str) -> Value {
    let url = tool_use
        .input
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");

    let content = render_template(webfetch_prompt, &serde_json::json!({"url": url}));

    serde_json::json!({
        "type": "tool_result",
        "tool_use_id": tool_use.id,
        "content": content,
    })
}

/// Generate a fail tool_result (is_error: true) for a rejected tool call.
pub(super) fn build_fail_result(tool_use: &ToolUse) -> Value {
    serde_json::json!({
        "type": "tool_result",
        "tool_use_id": tool_use.id,
        "is_error": true,
        "content": "The user doesn't want to proceed with this tool use. The tool use was rejected. Web fetch tools are not available through this proxy.",
    })
}
