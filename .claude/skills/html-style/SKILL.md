---
name: html-style
description: HTML/CSS conventions for this repo's server-rendered dashboard pages. Use when writing or modifying pages in pages/src/.
user-invocable: false
---

# Dashboard page conventions

## Framework

Pages use **`templates::Page`** for standard sections and **Leptos SSR** (`view!` macro) for custom content. Every page function returns `String` via `Page { ... }.render()`. **No external CSS, no JS libraries, no CDN links.**

```rust
use leptos::prelude::*;
use templates::{Breadcrumb, NavLink, Page};

pub fn render_example_view() -> String {
    let content = view! {
        <h2>"Items"</h2>
        <p>"Custom content here"</p>
    };

    Page {
        title: "Gateway Proxy - Example".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::current("Example"),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}
```

### `Page` struct

| Field | Type | Purpose |
|-------|------|---------|
| `title` | `String` | Browser tab title: `"Gateway Proxy - {context}"` |
| `breadcrumbs` | `Vec<Breadcrumb>` | `<h1>` breadcrumb trail |
| `nav_links` | `Vec<NavLink>` | Navigation section links |
| `info_rows` | `Vec<InfoRow>` | Key-value info table |
| `content` | `C: IntoView` | Custom page content (Leptos view or `()`) |
| `subpages` | `Vec<Subpage>` | Subpages table with Page/Count columns |

Renders in order: breadcrumbs, nav_links, info_rows, content, subpages. Empty sections are omitted. When content is `()`, use `..Default::default()` to fill remaining fields. When content is a view, all fields must be specified explicitly.

### All helpers (single reference)

| Helper | Purpose |
|--------|---------|
| `Breadcrumb::link(label, href)` | Breadcrumb link |
| `Breadcrumb::current(label)` | Terminal breadcrumb (plain text) |
| `NavLink::new(label, href)` | Navigation link |
| `NavLink::back()` | "Back" via `javascript:history.back()` — always last nav link |
| `InfoRow::new(label, value)` | Auto-escaped text value |
| `InfoRow::raw(label, html)` | Raw HTML (via `inner_html`) |
| `InfoRow::view(label, view)` | Leptos view value |
| `Subpage::new(label, href, count)` | Subpage entry — count accepts any `Display` |
| `collapsible_block(content, css_class)` | Show-more/show-less for content > 200 chars. Returns `AnyView`. Re-exported via `pages/src/lib.rs` |
| `Pagination::new(page, total_items, per_page, base_url, extra_params)` | Builds pagination state (computes `total_pages`) |
| `pagination_nav(&pagination)` | "Page X of Y" with Previous/Next links. Returns `AnyView` (empty when `total_pages <= 1`) |
| `render_kv_table(json_str)` | JSON object as Key/Value table with collapsible values (`detail/common.rs`) |
| `render_response_headers(req)` | Response status + headers as Key/Value table (`detail/common.rs`) |

## Leptos `view!` syntax

- Text nodes: `"Click me"`
- Interpolation: `{variable}` or `{expression}`
- Conditionals: `Either::Left(view! { ... })` / `Either::Right(())` to hide
- Optional: `Some(view! { ... })` / `None`
- Iteration: `.into_iter().map(|item| view! { ... }).collect::<Vec<_>>()`
- Type-erased: `.into_any()` returns `AnyView`
- Raw HTML: `<div inner_html={html_string}/>` (only for external HTML)
- Self-closing tags required: `<input />`, `<meta />`, `<br />`

**Never use `format!()` to build HTML.** Helper functions return `AnyView`, not `String`. The only `.to_html()` call is in `Page::render()`.

## CSS

Single `<style>` block in `templates::page_layout()` (`templates/src/lib.rs`). **No external CSS, no inline styles.**

```css
body { font-family: monospace; padding: 16px; }
table { width: 100%; border-collapse: collapse; }
th { text-align: left; padding: 6px 8px; border-bottom: 1px solid #ccc; }
td { padding: 6px 8px; border-bottom: 1px solid #eee; vertical-align: top; }
tr:last-child td { border-bottom: none; }
pre { white-space: pre-wrap; }
form { display: inline; }
details.collapsible { display: flex; flex-direction: column; }
details.collapsible > summary { cursor: pointer; list-style: none; order: 1; }
details.collapsible > summary::-webkit-details-marker { display: none; }
details.collapsible > summary .show-less { display: none; }
details.collapsible > .collapsible-full { white-space: pre-wrap; word-break: break-word; order: 0; }
details.collapsible[open] > summary .preview-text { display: none; }
details.collapsible[open] > summary .show-more { display: none; }
details.collapsible[open] > summary .show-less { display: inline; }
.hidden { display: none; }
.filtered-row { opacity: 0.45; }
.filtered-badge { color: #888; font-weight: bold; font-size: 0.85em; }
```

Colors: header borders `#ccc`, cell borders `#eee`, filtered text `#888`, everything else browser defaults. Add new classes to this style block.

## Routing and page types

```
/_dashboard/{entity}                              # Index — list all items
/_dashboard/{entity}/new                          # GET: new form / POST: create
/_dashboard/{entity}/{id}                         # Detail — show item + subpage links
/_dashboard/{entity}/{id}/edit                    # GET: edit form / POST: update
/_dashboard/{entity}/{id}/delete                  # POST: delete
/_dashboard/{entity}/{id}/{sub}                   # GET: subpage list / POST: add
/_dashboard/{entity}/{id}/{sub}/new               # GET: add form
/_dashboard/{entity}/{id}/{sub}/{item_id}/edit    # GET: edit form / POST: update
/_dashboard/{entity}/{id}/{sub}/{item_id}/delete  # POST: delete
```

| Page type | Key fields |
|-----------|------------|
| **Index** | breadcrumbs, nav_links ("New {Item}", Back), content = list table with total |
| **Detail** | breadcrumbs, nav_links ("Edit", Back), info_rows, subpages |
| **Edit** | breadcrumbs, nav_links (Back), content = pre-filled form, "Save" submit |
| **Subpage list** | breadcrumbs, nav_links ("New", Back), content = items table with total |
| **Content subpage** | breadcrumbs, nav_links (Back), content = total + page content |

### Breadcrumbs and nav

All ancestors are links; terminal page is plain text (`Breadcrumb::current`). Home page has no nav_links, only subpages. `NavLink::back()` is always last.

### Title format

`"Gateway Proxy - Home"`, `"Gateway Proxy - Sessions"`, `"Gateway Proxy - Session {name} - Requests"`, etc.

## Tables

### List table

```rust
<table>
    <tr><th>"ID"</th><th>"Name"</th><th></th></tr>
    {items.into_iter().map(|item| {
        view! {
            <tr>
                <td><a href={href}>{item.id.to_string()}</a></td>
                <td>{item.name}</td>
                <td>
                    <a href={edit_href}>"Edit"</a>
                    " "
                    <form method="POST" action={delete_action}>
                        <button type="submit">"Delete"</button>
                    </form>
                </td>
            </tr>
        }
    }).collect::<Vec<_>>()}
</table>
```

Action column: last `<th>` is empty, actions separated by `" "` (space text node).

### Key-value table

Prefer `InfoRow` in `Page` struct. For custom tables in content: `<tr><td>"Label"</td><td>{value}</td></tr>`.

## Forms

```rust
<form method="POST" action={form_action}>
    <table>
        <tr><td><label>"Name"</label></td><td><input type="text" name="name" required size="60"/></td></tr>
        <tr><td><label>"Checkbox"</label></td><td><input type="checkbox" name="field" value="1"/></td></tr>
        <tr><td></td><td><input type="submit" value="Create"/></td></tr>
    </table>
</form>
```

- Text inputs: `size="60"`. Checkboxes: `value="1"`, `checked={bool}` for edits
- Submit labels: "Create" (new), "Save" (edit), "Add" (sub-items)
- Placeholders only for format hints (URLs, tokens)
- Delete/edit always use dedicated routes, never query params
- Standalone actions: `<form method="POST" action={url}><button type="submit">"Label"</button></form>`

### Suggestion tables

"New item" pages can show suggested defaults. Each suggestion: `<code>` value + "Add" button with hidden input.

## List page features

### Auto-refresh

Meta refresh tag (3s) in content. Toggle: `?refresh=on` / `?refresh=off`, link text `"Enable auto-refresh"` / `"Disable auto-refresh"`.

```rust
{if auto_refresh {
    Some(view! { <meta http-equiv="refresh" content="3"/> })
} else {
    None
}}
```

### Pagination

Handler parses `?page=N` (default 1), runs COUNT + paginated SELECT (LIMIT/OFFSET), passes `Pagination` to render function.

```rust
use templates::{pagination_nav, Pagination};

let nav_top = pagination_nav(pagination);
let nav_bottom = pagination_nav(pagination);

let content = view! {
    <h2>"Items"</h2>
    <p>{format!("Total: {}", pagination.total_items)}</p>
    {nav_top}
    // ... table or empty message ...
    {nav_bottom}
};
```

- Use `pagination.total_items` for "Total: N" (not `items.len()`)
- Call `pagination_nav` twice for top/bottom nav (renders nothing when single page)
- `extra_params` preserves other query params (e.g. `"&refresh=on"`)

### Display controls

| Control | Query param | Toggle text |
|---------|-------------|-------------|
| Message order | `?order=asc` / `?order=desc` | `"Showing: newest first"` / `"oldest first"` |
| JSON truncation | `?truncate=on` / `?truncate=off` | `"Show full strings"` / `"Show truncated"` |

### Dimmed rows

`.filtered-row` for opacity, `.filtered-badge` for `"[FILTERED]"` label.

## Content display

| Strategy | When | Implementation |
|----------|------|----------------|
| Key-value table | JSON objects | `render_kv_table()` with collapsible values |
| Pre-formatted | Raw JSON, small blocks | `view! { <pre>{json_str}</pre> }` |
| Readonly textarea | Full request JSON | `<textarea readonly rows="30" cols="80" wrap="off">{json}</textarea>` |
| Collapsible block | Long text in cells | `collapsible_block(text, "")` (200-char threshold) |

Text preview truncation: `text.replace('\n', " ")` + `"..."` suffix.

All dynamic text is auto-escaped by Leptos. **Never manually escape.** Only exception: `page_layout()` escapes `<title>` internally.

## JavaScript

Only two uses. Everything else is server-side (forms + redirects).

1. **Copy-to-clipboard**: `navigator.clipboard.writeText()` via onclick. Used with `InfoRow::view("Label", render_copy_link(&url))`.
2. **Back navigation**: `NavLink::back()` renders `javascript:history.back()`.

## Status indicators

Simple text, no special styling. Timestamps as-is from DB. Empty values: `String::new()` or `.unwrap_or_default()`.
