pub mod detail;
pub mod filters;
pub mod home;
pub mod requests;
pub mod session_show;
pub mod sessions;

pub fn page_layout(title: &str, body_html: String) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
body {{ font-family: monospace; padding: 16px; }}
table {{ width: 100%; border-collapse: collapse; }}
th {{ text-align: left; padding: 6px 8px; border-bottom: 1px solid #ccc; }}
td {{ padding: 6px 8px; border-bottom: 1px solid #eee; vertical-align: top; }}
tr:last-child td {{ border-bottom: none; }}
pre {{ white-space: pre-wrap; }}
form {{ display: inline; }}
details.collapsible {{ display: flex; flex-direction: column; }}
details.collapsible > summary {{ cursor: pointer; list-style: none; order: 1; }}
details.collapsible > summary::-webkit-details-marker {{ display: none; }}
details.collapsible > summary .show-less {{ display: none; }}
details.collapsible > .collapsible-full {{ white-space: pre-wrap; word-break: break-word; order: 0; }}
details.collapsible[open] > summary .preview-text {{ display: none; }}
details.collapsible[open] > summary .show-more {{ display: none; }}
details.collapsible[open] > summary .show-less {{ display: inline; }}
.hidden {{ display: none; }}
.filtered-row {{ opacity: 0.45; }}
.filtered-badge {{ color: #888; font-weight: bold; font-size: 0.85em; }}
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
    if content.len() <= COLLAPSE_THRESHOLD {
        if content.contains('\n') {
            return format!(r#"<pre class="{}">{}</pre>"#, css_class, escaped);
        } else {
            return format!(r#"<div class="{}">{}</div>"#, css_class, escaped);
        }
    }
    let preview: String = content.chars().take(COLLAPSE_THRESHOLD).collect();
    let preview_escaped = html_escape(&preview);
    format!(
        r#"<details class="collapsible"><summary><span class="preview-text {cls}">{preview}...</span> <span class="show-more">show more</span><span class="show-less">show less</span></summary><div class="collapsible-full {cls}">{full}</div></details>"#,
        cls = css_class,
        preview = preview_escaped,
        full = escaped
    )
}
