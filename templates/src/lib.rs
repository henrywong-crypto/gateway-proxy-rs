use leptos::either::Either;
use leptos::prelude::*;

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

pub struct Breadcrumb {
    pub label: String,
    pub href: Option<String>,
}

impl Breadcrumb {
    pub fn link(label: impl ToString, href: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            href: Some(href.to_string()),
        }
    }

    pub fn current(label: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            href: None,
        }
    }
}

pub struct NavLink {
    pub label: String,
    pub href: String,
}

impl NavLink {
    pub fn new(label: impl ToString, href: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            href: href.to_string(),
        }
    }

    pub fn back() -> Self {
        Self {
            label: "Back".to_string(),
            href: "javascript:history.back()".to_string(),
        }
    }
}

pub struct InfoRow {
    pub label: String,
    pub value: String,
}

impl InfoRow {
    pub fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: html_escape(value),
        }
    }

    pub fn raw(label: &str, value: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
        }
    }
}

pub struct Subpage {
    pub label: String,
    pub href: String,
    pub count: String,
}

impl Subpage {
    pub fn new(label: impl ToString, href: impl ToString, count: impl std::fmt::Display) -> Self {
        Self {
            label: label.to_string(),
            href: href.to_string(),
            count: count.to_string(),
        }
    }
}

pub struct Page<C: IntoView = ()> {
    pub title: String,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub nav_links: Vec<NavLink>,
    pub info_rows: Vec<InfoRow>,
    pub content: C,
    pub subpages: Vec<Subpage>,
}

impl Default for Page {
    fn default() -> Self {
        Page {
            title: String::new(),
            breadcrumbs: Vec::new(),
            nav_links: Vec::new(),
            info_rows: Vec::new(),
            content: (),
            subpages: Vec::new(),
        }
    }
}

impl<C: IntoView> Page<C> {
    pub fn render(self) -> String {
        let Page {
            title,
            breadcrumbs,
            nav_links,
            info_rows,
            content,
            subpages,
        } = self;

        let body = view! {
            {if !breadcrumbs.is_empty() {
                Either::Left(view! {
                    <h1>
                        {breadcrumbs.into_iter().enumerate().map(|(i, crumb)| {
                            let sep = if i > 0 { " / " } else { "" };
                            match crumb.href {
                                Some(href) => Either::Left(view! {
                                    {sep}<a href={href}>{crumb.label}</a>
                                }),
                                None => Either::Right(view! {
                                    {sep}{crumb.label}
                                }),
                            }
                        }).collect::<Vec<_>>()}
                    </h1>
                })
            } else {
                Either::Right(())
            }}

            {if !nav_links.is_empty() {
                Either::Left(view! {
                    <h2>"Navigation"</h2>
                    <table>
                        {nav_links.into_iter().map(|link| {
                            view! { <tr><td><a href={link.href}>{link.label}</a></td></tr> }
                        }).collect::<Vec<_>>()}
                    </table>
                })
            } else {
                Either::Right(())
            }}

            {if !info_rows.is_empty() {
                Either::Left(view! {
                    <h2>"Info"</h2>
                    <table>
                        {info_rows.into_iter().map(|row| {
                            view! { <tr><td>{row.label}</td><td inner_html={row.value}></td></tr> }
                        }).collect::<Vec<_>>()}
                    </table>
                })
            } else {
                Either::Right(())
            }}

            {content}

            {if !subpages.is_empty() {
                Either::Left(view! {
                    <h2>"Subpages"</h2>
                    <table>
                        <tr><th>"Page"</th><th>"Count"</th></tr>
                        {subpages.into_iter().map(|sp| {
                            view! { <tr><td><a href={sp.href}>{sp.label}</a></td><td>{sp.count}</td></tr> }
                        }).collect::<Vec<_>>()}
                    </table>
                })
            } else {
                Either::Right(())
            }}
        };

        page_layout(&title, body.to_html())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_special_chars() {
        assert_eq!(
            html_escape("<b>\"a&b\"</b>"),
            "&lt;b&gt;&quot;a&amp;b&quot;&lt;/b&gt;"
        );
    }

    #[test]
    fn html_escape_no_special_chars() {
        assert_eq!(html_escape("hello world"), "hello world");
    }

    #[test]
    fn collapsible_block_short_single_line() {
        let result = collapsible_block("short text", "cls");
        assert_eq!(result, r#"<div class="cls">short text</div>"#);
    }

    #[test]
    fn collapsible_block_short_multiline() {
        let result = collapsible_block("line1\nline2", "cls");
        assert_eq!(
            result,
            r#"<pre class="cls">line1
line2</pre>"#
        );
    }

    #[test]
    fn collapsible_block_long_content() {
        let long = "a".repeat(300);
        let result = collapsible_block(&long, "cls");
        assert!(result.contains("show more"));
        assert!(result.contains("show less"));
        assert!(result.contains("collapsible"));
    }

    #[test]
    fn page_layout_wraps_body() {
        let result = page_layout("Test Title", "<p>body</p>".to_string());
        assert!(result.contains("<title>Test Title</title>"));
        assert!(result.contains("<p>body</p>"));
        assert!(result.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn page_layout_escapes_title() {
        let result = page_layout("<script>", "".to_string());
        assert!(result.contains("<title>&lt;script&gt;</title>"));
    }

    #[test]
    fn page_render_breadcrumbs_only() {
        let html = Page {
            title: "Test".to_string(),
            breadcrumbs: vec![
                Breadcrumb::link("Home", "/"),
                Breadcrumb::current("Current"),
            ],
            nav_links: vec![],
            info_rows: vec![],
            content: (),
            subpages: vec![],
        }
        .render();
        assert!(html.contains("<h1>"));
        assert!(html.contains(r#"<a href="/">"#));
        assert!(html.contains("Home"));
        assert!(html.contains(" / "));
        assert!(html.contains("Current"));
        assert!(html.contains("</h1>"));
    }

    #[test]
    fn page_render_nav_links() {
        let html = Page {
            title: "Test".to_string(),
            breadcrumbs: vec![],
            nav_links: vec![NavLink::new("Edit", "/edit"), NavLink::back()],
            info_rows: vec![],
            content: (),
            subpages: vec![],
        }
        .render();
        assert!(html.contains("<h2>Navigation</h2>"));
        assert!(html.contains(r#"<a href="/edit">"#));
        assert!(html.contains("Edit"));
        assert!(html.contains(r#"<a href="javascript:history.back()">"#));
        assert!(html.contains("Back"));
    }

    #[test]
    fn page_render_info_rows_escaped() {
        let html = Page {
            title: "Test".to_string(),
            breadcrumbs: vec![],
            nav_links: vec![],
            info_rows: vec![InfoRow::new("Key", "<value>")],
            content: (),
            subpages: vec![],
        }
        .render();
        assert!(html.contains("<h2>Info</h2>"));
        assert!(html.contains("Key"));
        assert!(html.contains("&lt;value&gt;"));
        assert!(!html.contains("<value>"));
    }

    #[test]
    fn page_render_info_rows_raw() {
        let html = Page {
            title: "Test".to_string(),
            breadcrumbs: vec![],
            nav_links: vec![],
            info_rows: vec![InfoRow::raw("Key", "<b>bold</b>")],
            content: (),
            subpages: vec![],
        }
        .render();
        assert!(html.contains("<b>bold</b>"));
    }

    #[test]
    fn page_render_content_view() {
        let html = Page {
            title: "Test".to_string(),
            breadcrumbs: vec![],
            nav_links: vec![],
            info_rows: vec![],
            content: view! { <form><input type="text" name="x"/></form> },
            subpages: vec![],
        }
        .render();
        assert!(html.contains("<form>"));
        assert!(html.contains(r#"name="x""#));
    }

    #[test]
    fn page_render_subpages() {
        let html = Page {
            title: "Test".to_string(),
            breadcrumbs: vec![],
            nav_links: vec![],
            info_rows: vec![],
            content: (),
            subpages: vec![Subpage::new("Requests", "/requests", 42)],
        }
        .render();
        assert!(html.contains("<h2>Subpages</h2>"));
        assert!(html.contains("Page"));
        assert!(html.contains("Count"));
        assert!(html.contains(r#"<a href="/requests">"#));
        assert!(html.contains("Requests"));
        assert!(html.contains("42"));
    }

    #[test]
    fn page_render_empty_sections_omitted() {
        let html = Page {
            title: "Test".to_string(),
            breadcrumbs: vec![],
            nav_links: vec![],
            info_rows: vec![],
            content: (),
            subpages: vec![],
        }
        .render();
        assert!(!html.contains("<h1>"));
        assert!(!html.contains("Navigation"));
        assert!(!html.contains("Info"));
        assert!(!html.contains("Subpages"));
    }

    #[test]
    fn page_render_full() {
        let html = Page {
            title: "Full Page".to_string(),
            breadcrumbs: vec![Breadcrumb::link("Home", "/"), Breadcrumb::current("Detail")],
            nav_links: vec![NavLink::back()],
            info_rows: vec![InfoRow::new("Name", "test")],
            content: view! { <p>"content"</p> },
            subpages: vec![Subpage::new("Sub", "/sub", 5)],
        }
        .render();
        assert!(html.contains("<title>Full Page</title>"));
        assert!(html.contains("<h1>"));
        assert!(html.contains("Navigation"));
        assert!(html.contains("Info"));
        assert!(html.contains("<p>"));
        assert!(html.contains("content"));
        assert!(html.contains("Subpages"));
    }
}
