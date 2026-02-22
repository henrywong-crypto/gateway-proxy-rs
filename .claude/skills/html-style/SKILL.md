---
name: html-style
description: HTML/CSS conventions for this repo's server-rendered dashboard pages. Use when writing or modifying pages in server/src/pages/.
user-invocable: false
---

# Dashboard page conventions

## Framework

Pages use the **`templates::Page`** struct for standard page sections (breadcrumbs, navigation, info rows, subpages) and **Leptos SSR** (`view!` macro) for custom content. **No external CSS frameworks, no JavaScript libraries, no CDN links.** Everything is self-contained.

```rust
use leptos::prelude::*;
use templates::{Breadcrumb, NavLink, Page};

pub fn render_example() -> String {
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

Every page function returns `String` by constructing a `Page` and calling `.render()`.

### `Page` struct fields

| Field | Type | Purpose |
|-------|------|---------|
| `title` | `String` | Browser tab title |
| `breadcrumbs` | `Vec<Breadcrumb>` | `<h1>` breadcrumb trail |
| `nav_links` | `Vec<NavLink>` | Navigation section links |
| `info_rows` | `Vec<InfoRow>` | Key-value info table |
| `content` | `C: IntoView` | Custom page content (Leptos view or `()`) |
| `subpages` | `Vec<Subpage>` | Subpages table with Page/Count columns |

Empty sections are automatically omitted from the rendered HTML.

### Pages without custom content

When a page only needs the standard sections (no custom content), use `Page<()>` with `..Default::default()`:

```rust
Page {
    title: "Gateway Proxy - Home".to_string(),
    breadcrumbs: vec![Breadcrumb::current("Home")],
    subpages: vec![
        Subpage::new("Sessions", "/_dashboard/sessions", session_count),
    ],
    ..Default::default()
}
.render()
```

**Important:** `..Default::default()` only works when the `content` field is `()` (omitted). When you provide a `content` view, you must specify all fields explicitly — `Default` is not implemented for `Page<SomeViewType>`.

### Builder helpers

| Helper | Purpose |
|--------|---------|
| `Breadcrumb::link(label, href)` | Breadcrumb item that is a link |
| `Breadcrumb::current(label)` | Terminal breadcrumb item (plain text) |
| `NavLink::new(label, href)` | Navigation link |
| `NavLink::back()` | "Back" link using `javascript:history.back()` |
| `InfoRow::new(label, value)` | Info row with HTML-escaped value |
| `InfoRow::raw(label, value)` | Info row with raw HTML value (for copy links etc.) |
| `Subpage::new(label, href, count)` | Subpage table entry — count accepts any `Display` type |

## Leptos `view!` syntax

- String literals as text nodes: `"Click me"`
- Variable interpolation: `{variable}` or `{expression}`
- Conditionals: `Either::Left(view! { ... })` / `Either::Right(view! { ... })`
- Conditionally hidden section: `Either::Right(())`
- Optional elements: `Some(view! { ... })` / `None`
- Iteration: `.into_iter().map(|item| view! { ... }).collect::<Vec<_>>()`
- Pre-rendered HTML strings: `<div inner_html={html_string}/>` or `<span inner_html={html_string}/>`
- All self-closing tags must use `/>` syntax (Leptos requirement): `<input />`, `<meta />`, `<br />`

### When to use `view!` vs raw `format!()`

Use the `view!` macro for `Page` content fields and standard page sections. Use raw `format!()` string building for complex, data-driven table rendering where the HTML structure is deeply conditional or involves nested iteration. Functions that return raw HTML strings are injected via `<div inner_html={html_string}/>`.

## CSS

Single embedded `<style>` block in `templates::page_layout()` (`templates/src/lib.rs`). **No external CSS, no inline styles.** Monospace font, table-based layout, minimal borders.

### Complete CSS reference

```css
body { font-family: monospace; padding: 16px; }
table { width: 100%; border-collapse: collapse; }
th { text-align: left; padding: 6px 8px; border-bottom: 1px solid #ccc; }
td { padding: 6px 8px; border-bottom: 1px solid #eee; vertical-align: top; }
tr:last-child td { border-bottom: none; }
pre { white-space: pre-wrap; }
form { display: inline; }

/* Collapsible blocks (show-more/show-less) */
details.collapsible { display: flex; flex-direction: column; }
details.collapsible > summary { cursor: pointer; list-style: none; order: 1; }
details.collapsible > summary::-webkit-details-marker { display: none; }
details.collapsible > summary .show-less { display: none; }
details.collapsible > .collapsible-full { white-space: pre-wrap; word-break: break-word; order: 0; }
details.collapsible[open] > summary .preview-text { display: none; }
details.collapsible[open] > summary .show-more { display: none; }
details.collapsible[open] > summary .show-less { display: inline; }

/* Utility */
.hidden { display: none; }

/* Filtered items (dimmed rows with badge) */
.filtered-row { opacity: 0.45; }
.filtered-badge { color: #888; font-weight: bold; font-size: 0.85em; }
```

### Color palette

| Usage | Color |
|-------|-------|
| Header borders | `#ccc` |
| Cell borders | `#eee` |
| Filtered text | `#888` |
| Text | Default black |
| Background | Default white |
| Links | Default browser blue |
| Filtered row opacity | `0.45` |

### Adding new styles

Add new CSS classes to the style block in `templates::page_layout()` (`templates/src/lib.rs`). Never use inline styles. Never add external CSS.

## Shared helpers

### Templates crate (`templates/src/lib.rs`)

| Helper | Purpose |
|--------|---------|
| `Page { ... }.render()` | Renders a full HTML page with standard sections |
| `page_layout(title, body_html)` | Low-level wrapper — used internally by `Page::render()` |
| `html_escape(s)` | Escapes `& < > "` for use in raw HTML strings |
| `collapsible_block(content, css_class)` | Show-more/show-less for content > 200 chars |

Re-exported by `server/src/pages/mod.rs`:
```rust
pub use templates::{collapsible_block, html_escape};
```

### `collapsible_block()` behavior

- **Threshold**: 200 characters
- **Short content with newlines**: Wraps in `<pre class="{css_class}">`
- **Short content without newlines**: Wraps in `<div class="{css_class}">`
- **Long content**: Creates a `<details class="collapsible">` element with preview (first 200 chars + "..."), "show more" / "show less" toggle
- The `css_class` parameter is applied to both the preview and full content spans
- All content is HTML-escaped automatically

### Detail page helpers (`server/src/pages/detail/common.rs`)

| Helper | Purpose |
|--------|---------|
| `render_kv_table(json_str)` | Renders a JSON object as a Key/Value table with collapsible values |
| `render_response_headers(req)` | Renders response status + headers as Key/Value table |

## Page structure pattern

The `Page` struct renders sections in this order (empty sections are omitted):

1. `breadcrumbs` → `<h1>` breadcrumb trail
2. `nav_links` → `<h2>"Navigation"</h2>` with action links
3. `info_rows` → `<h2>"Info"</h2>` with key-value table
4. `content` → custom page content (views, forms, tables, etc.)
5. `subpages` → `<h2>"Subpages"</h2>` with Page/Count table

Custom `<h2>` sections (e.g., "Actions", list headings with totals) go in the `content` field.

### Page title format

Browser tab titles follow the pattern: `"Gateway Proxy - {context}"`. Context is hierarchical, e.g.:
- `"Gateway Proxy - Home"`
- `"Gateway Proxy - Sessions"`
- `"Gateway Proxy - Session {name}"`
- `"Gateway Proxy - Session {name} - Requests"`
- `"Gateway Proxy - Edit Session {name}"`

### List -> Detail -> Subpage navigation

All top-level entities follow a consistent routing and page pattern:

```
/_dashboard/{entity}                              # Index — list all items
/_dashboard/{entity}/new                          # GET: new form / POST: create
/_dashboard/{entity}/{id}                         # Detail — show single item with subpage links
/_dashboard/{entity}/{id}/edit                    # GET: edit form / POST: update
/_dashboard/{entity}/{id}/delete                  # POST: delete
/_dashboard/{entity}/{id}/{sub}                   # GET: subpage list / POST: add item
/_dashboard/{entity}/{id}/{sub}/new               # GET: add form for subpage items
/_dashboard/{entity}/{id}/{sub}/{item_id}/edit    # GET: edit form / POST: update item
/_dashboard/{entity}/{id}/{sub}/{item_id}/delete  # POST: delete item
```

**Index page**: breadcrumbs, nav_links ("New {Item}", Back), content = item list table with total count.

**Detail page**: breadcrumbs, nav_links ("Edit {Item}", Back), info_rows, subpages. May include conditional actions in content.

**Edit page**: breadcrumbs (parent is a link), nav_links (Back), content = form with current values pre-filled, "Save" submit.

**Subpage list** (`/{sub}`): breadcrumbs (parent detail page is a link), nav_links ("New {Item}", Back), content = items table with total count. Each row shows item fields, "Edit" link, and Delete button. The add form lives on the separate `/new` subpage.

**Subpage edit** (`/{sub}/{item_id}/edit`): breadcrumbs (parent subpage list is a link), nav_links (Back), content = edit form. POST redirects back to the list.

**Subpage add** (`/{sub}/new`): breadcrumbs (parent subpage list is a link), nav_links (Back), content = add form and optional suggested defaults. POST submits to the parent subpage list URL.

**Content subpage**: breadcrumbs, nav_links (Back), content = total count + page content. Leaf pages with no subpages.

### Breadcrumbs

```rust
breadcrumbs: vec![
    Breadcrumb::link("Home", "/_dashboard"),
    Breadcrumb::link("Items", "/_dashboard/items"),
    Breadcrumb::current(format!("Item {}", item.name)),
],
```

All ancestors are links; the current (terminal) page is plain text. Separated by `" / "`.

### Navigation links

```rust
nav_links: vec![
    NavLink::new("New Item", "/_dashboard/items/new"),
    NavLink::back(),  // always last
],
```

The Home page has no nav_links — it only has subpages.

### Content field

Custom page content goes in `content`. This is where forms, list tables, controls, and conditional actions live:

```rust
let content = view! {
    <h2>"Items"</h2>
    <p>{format!("Total: {}", items.len())}</p>
    {if items.is_empty() {
        Either::Left(view! { <p>"No items yet."</p> })
    } else {
        Either::Right(view! { <table>...</table> })
    }}
};

Page {
    // ...standard fields...
    content,
    // ...
}
.render()
```

## Tables

Tables are the primary layout primitive. Types:

### Key-value info table

Prefer using `InfoRow` in the `Page` struct for standard info sections. For custom key-value tables in content, use:

```rust
<table>
    <tr><td>"Label"</td><td>{value}</td></tr>
</table>
```

### List table (with headers)

```rust
<table>
    <tr>
        <th>"ID"</th>
        <th>"Name"</th>
        <th>"Created"</th>
        <th></th>  // empty header for action column
    </tr>
    {items.into_iter().map(|item| {
        view! {
            <tr>
                <td><a href={href}>{item.id.to_string()}</a></td>
                <td>{item.name}</td>
                <td>{item.created_at.unwrap_or_default()}</td>
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

### Nested tables

Tables can be nested for structured data. No special CSS needed — nested tables inherit the same styles.

### Action column pattern

The last column in list tables holds inline actions. Multiple actions are separated by `" "` (space text node). Forms use `display: inline` (set globally in CSS):

```rust
<td>
    <a href={edit_href}>"Edit"</a>
    " "
    <form method="POST" action={delete_action}>
        <button type="submit">"Delete"</button>
    </form>
</td>
```

## Forms

- `method="POST"` with `action` URL
- Inputs inside `<table>` rows: `<td><label>` / `<td><input>`
- Submit button in its own row with empty first `<td>`:

```rust
<form method="POST" action={form_action}>
    <table>
        <tr>
            <td><label>"Name"</label></td>
            <td><input type="text" name="name" required size="60"/></td>
        </tr>
        <tr>
            <td><label>"Checkbox Field"</label></td>
            <td><input type="checkbox" name="field" value="1"/></td>
        </tr>
        <tr>
            <td></td>
            <td><input type="submit" value="Create"/></td>
        </tr>
    </table>
</form>
```

### Form conventions

- All `<input>` tags must be self-closing (`<input ... />`) — Leptos `view!` requires this
- Text inputs use `size="60"` consistently
- Number inputs use `type="number"` with `min` attribute where appropriate
- Checkboxes use `value="1"` and `checked={bool_value}` for edit forms
- Placeholders only where they hint at expected format (e.g. URLs, auth tokens). Do not add placeholders on simple name or pattern fields
- Delete and edit actions always use dedicated routes (`/{item_id}/delete`, `/{item_id}/edit`), never query parameters
- Submit labels: "Create" for new items, "Save" for edits, "Add" for sub-items
- Standalone action forms (not in tables): `<form method="POST" action={url}><button type="submit">"Label"</button></form>`

### Suggestion tables

"New item" pages can show suggested defaults below the form. Existing items are filtered out. Each suggestion has the value in `<code>` and an "Add" button that submits a hidden input:

```rust
{if has_suggestions {
    Either::Left(view! {
        <h2>"Suggested Items"</h2>
        <table>
            {suggestions.into_iter().map(|s| {
                let value = s.to_string();
                view! {
                    <tr>
                        <td><code>{value.clone()}</code></td>
                        <td>
                            <form method="POST" action={form_action.clone()}>
                                <input type="hidden" name="field_name" value={value}/>
                                <button type="submit">"Add"</button>
                            </form>
                        </td>
                    </tr>
                }
            }).collect::<Vec<_>>()}
        </table>
    })
} else {
    Either::Right(())
}}
```

## No JavaScript (with two exceptions)

All interactivity is server-side (HTML forms + redirects). The only JavaScript is:

### 1. Copy-to-clipboard

Uses the native `navigator.clipboard.writeText()` API via onclick:

```rust
fn copy_link_html(text: &str) -> String {
    format!(
        r#" <a href="javascript:void(0)" onclick="navigator.clipboard.writeText('{}')">Copy</a>"#,
        html_escape(text)
    )
}
```

Injected via `InfoRow::raw()` after the displayed URL text.

### 2. Back navigation

`NavLink::back()` renders `<a href="javascript:history.back()">Back</a>` — present on every page except Home, always the last nav link.

## Auto-refresh

Pages that monitor live data use a meta refresh tag (3-second interval). Placed inside the `content` field:

```rust
let content = view! {
    {if auto_refresh {
        Some(view! { <meta http-equiv="refresh" content="3"/> })
    } else {
        None
    }}
    // ... rest of content
};
```

Toggle link text: `"Enable auto-refresh"` / `"Disable auto-refresh"`.
Query parameter: `?refresh=on` / `?refresh=off`.

## Content display controls

Content pages may have toggle links above the content for display options:

| Control | Query param | Toggle text |
|---------|-------------|-------------|
| Message order | `?order=asc` / `?order=desc` | `"Showing: newest first"` / `"oldest first"` with switch link |
| JSON truncation | `?truncate=on` / `?truncate=off` | `"Show full strings"` / `"Show truncated"` |
| Auto-refresh | `?refresh=on` / `?refresh=off` | `"Enable auto-refresh"` / `"Disable auto-refresh"` |

Controls are rendered as raw HTML strings and injected via `<div inner_html={controls_html}/>`.

## Dimmed rows and badges

For table rows that should appear visually de-emphasized, use the `.filtered-row` class for opacity and `.filtered-badge` for an inline label:

```rust
let row_class = if dimmed { " class=\"filtered-row\"" } else { "" };
let badge = if dimmed {
    " <span class=\"filtered-badge\">[FILTERED]</span>"
} else {
    ""
};

html.push_str(&format!("<tr{}><td>{}{}</td>...</tr>", row_class, html_escape(name), badge));
```

## JSON and code display

Rendering strategies based on data shape:

| Strategy | When to use | Implementation |
|----------|-------------|----------------|
| Key-value table | JSON objects | `render_kv_table()` — `<table>` with Key/Value headers, values use `collapsible_block()` |
| Pre-formatted text | Raw/unparseable JSON, small blocks | `<pre>{html_escape(json)}</pre>` |
| Readonly textarea | Full request JSON | `<textarea readonly rows="30" cols="80" wrap="off">{html_escape(json)}</textarea>` |
| Collapsible block | Long text in table cells | `collapsible_block(text, "")` (200-char threshold) |

## Text preview truncation

When showing previews of long content in list tables, truncate with `"..."` suffix. Replace newlines with spaces: `text.replace('\n', " ")`.

## HTML escaping

Always escape dynamic content in raw HTML strings using `html_escape()`. The function escapes `& < > "`. This is **not** needed inside `view!` macro text nodes (Leptos handles escaping), but **is** needed in:

- `format!()` strings that produce HTML
- Content passed to `inner_html`
- Arguments to `collapsible_block()`'s return value (it escapes internally)

## Status indicators

Simple text strings, no special styling:
- Active/inactive states: `"active"` / `"inactive"` / `"--"`
- Timestamps: Displayed as-is from the database, no client-side formatting
- Empty/missing values: `String::new()` or `.unwrap_or_default()`
