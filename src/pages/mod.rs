pub mod dashboard;
pub mod detail;
pub mod home;

pub fn page_layout(title: &str, body_html: String) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ font-family: monospace; background: #1e1e1e; color: #d4d4d4; padding: 16px; }}
h1 {{ margin-bottom: 12px; font-size: 18px; color: #569cd6; }}
h2 {{ margin-bottom: 10px; font-size: 16px; color: #569cd6; }}
a {{ color: #569cd6; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
th {{ text-align: left; color: #569cd6; padding: 6px 8px; border-bottom: 1px solid #444; }}
td {{ padding: 6px 8px; border-bottom: 1px solid #333; vertical-align: top; }}
tr:hover {{ background: #2a2d2e; }}
.method {{ color: #dcdcaa; font-weight: bold; }}
.path {{ color: #ce9178; }}
.model {{ color: #569cd6; }}
.time {{ color: #6a9955; font-size: 12px; }}
input[type=text] {{ background: #333; color: #d4d4d4; border: 1px solid #555; padding: 4px 8px; font-family: monospace; }}
button, input[type=submit] {{ background: #333; color: #d4d4d4; border: 1px solid #555; padding: 4px 12px; cursor: pointer; font-family: monospace; }}
button:hover, input[type=submit]:hover {{ background: #444; }}
.tab-bar {{ display: flex; gap: 4px; margin-bottom: 12px; }}
.tab {{ padding: 4px 10px; background: #333; border: 1px solid #555; font-size: 12px; color: #d4d4d4; }}
.tab.active {{ background: #0e639c; border-color: #0e639c; color: #fff; }}
pre {{ background: #1e1e1e; border: 1px solid #333; padding: 10px; overflow-x: auto; font-size: 13px; white-space: pre-wrap; word-break: break-all; max-height: 600px; overflow-y: auto; }}
.card {{ background: #252526; border: 1px solid #444; padding: 12px; margin-bottom: 8px; }}
.card h3 {{ color: #dcdcaa; font-size: 14px; margin-bottom: 6px; }}
.msg-card {{ background: #252526; border: 1px solid #444; padding: 10px; margin-bottom: 8px; }}
.msg-card.role-user {{ border-left: 3px solid #569cd6; }}
.msg-card.role-assistant {{ border-left: 3px solid #dcdcaa; }}
.msg-role {{ font-size: 11px; font-weight: bold; text-transform: uppercase; margin-bottom: 6px; }}
.msg-role.user {{ color: #569cd6; }}
.msg-role.assistant {{ color: #dcdcaa; }}
.msg-block {{ margin-bottom: 6px; padding: 6px 8px; background: #1e1e1e; font-size: 12px; line-height: 1.4; }}
.msg-block-label {{ font-size: 10px; color: #888; text-transform: uppercase; margin-bottom: 3px; }}
.msg-block-text {{ color: #d4d4d4; white-space: pre-wrap; word-break: break-all; }}
.msg-block-thinking {{ color: #6a9955; font-style: italic; white-space: pre-wrap; word-break: break-all; }}
.msg-block-tool {{ color: #ce9178; }}
.tool-name {{ color: #dcdcaa; font-weight: bold; }}
.msg-block-result {{ color: #9cdcfe; white-space: pre-wrap; word-break: break-all; }}
.kv-table td:first-child {{ color: #dcdcaa; white-space: nowrap; }}
.tool-desc {{ color: #9cdcfe; font-size: 12px; margin-bottom: 8px; line-height: 1.4; }}
.proxy-url {{ background: #252526; border: 1px solid #444; padding: 8px 12px; margin: 8px 0; font-size: 13px; }}
.form-row {{ margin-bottom: 8px; }}
.form-row label {{ display: inline-block; width: 100px; }}
.copy-btn {{ background: #0e639c; color: #fff; border: none; padding: 4px 10px; cursor: pointer; font-size: 12px; font-family: monospace; }}
.copy-btn:hover {{ background: #1177bb; }}
details.collapsible {{ background: #1e1e1e; }}
details.collapsible > summary {{ cursor: pointer; list-style: none; }}
details.collapsible > summary::-webkit-details-marker {{ display: none; }}
details.collapsible > summary .preview-text {{ white-space: pre-wrap; word-break: break-all; }}
details.collapsible > summary .show-more {{ color: #569cd6; font-size: 11px; }}
details.collapsible[open] > summary {{ display: none; }}
details.collapsible > .collapsible-full {{ white-space: pre-wrap; word-break: break-all; }}
</style>
</head>
<body>
{body_html}
</body>
</html>"#,
        title = html_escape(title),
        body_html = body_html
    )
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const COLLAPSE_THRESHOLD: usize = 200;

pub fn collapsible_block(content: &str, css_class: &str) -> String {
    let escaped = html_escape(content);
    if content.len() <= COLLAPSE_THRESHOLD && !content.contains('\n') {
        return format!(r#"<div class="{}">{}</div>"#, css_class, escaped);
    }
    let preview: String = content.chars().take(COLLAPSE_THRESHOLD).collect();
    let preview_escaped = html_escape(&preview);
    format!(
        r#"<details class="collapsible"><summary><span class="preview-text {cls}">{preview}...</span> <span class="show-more">show more</span></summary><div class="collapsible-full {cls}">{full}</div></details>"#,
        cls = css_class,
        preview = preview_escaped,
        full = escaped
    )
}
