use crate::pages::{collapsible_block, html_escape};

pub fn render_tools(json_str: &str) -> String {
    let Ok(tools) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
        return format!("<pre>{}</pre>", html_escape(json_str));
    };

    let mut html =
        String::from("<table><tr><th>Name</th><th>Description</th><th>Parameters</th></tr>");
    for tool in &tools {
        let name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("(unnamed)");
        let desc = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        let mut params_html = String::new();
        if let Some(schema) = tool.get("input_schema").and_then(|s| s.as_object()) {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                let required: Vec<&str> = schema
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();

                params_html.push_str(
                    "<table><tr><th>Name</th><th>Type</th><th>Req</th><th>Description</th></tr>",
                );
                for (k, v) in props {
                    let ptype = v.get("type").and_then(|t| t.as_str()).unwrap_or("object");
                    let pdesc = v.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let req = if required.contains(&k.as_str()) {
                        "yes"
                    } else {
                        "no"
                    };
                    params_html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(k),
                        html_escape(ptype),
                        req,
                        html_escape(pdesc)
                    ));
                }
                params_html.push_str("</table>");
            }
        }

        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(name),
            collapsible_block(desc, ""),
            params_html
        ));
    }
    html.push_str("</table>");
    html
}
