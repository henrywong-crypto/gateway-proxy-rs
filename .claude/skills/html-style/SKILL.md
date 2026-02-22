---
name: html-style
description: HTML/CSS conventions for this repo's server-rendered dashboard pages. Use when writing or modifying pages in server/src/pages/.
user-invocable: false
---

# Dashboard page conventions

## Framework

Pages use **Leptos SSR** (`view!` macro) — JSX-like syntax compiled to HTML strings on the server. **No external CSS frameworks, no JavaScript libraries, no CDN links.** Everything is self-contained.

```rust
use leptos::prelude::*;
use crate::pages::page_layout;

pub fn render_example(title: &str) -> String {
    let body = view! {
        <h1>"Page Title"</h1>
        <table>
            <tr><td>{title.to_string()}</td></tr>
        </table>
    };
    page_layout("Browser Tab Title", body.to_html())
}
```

Every page function returns `String` by calling `page_layout(title, body.to_html())`.

## Leptos `view!` syntax

- String literals as text nodes: `"Click me"`
- Variable interpolation: `{variable}` or `{expression}`
- Conditionals: `Either::Left(view! { ... })` / `Either::Right(view! { ... })`
- Conditionally hidden section: `Either::Right(view! {})` (empty view)
- Optional elements: `Some(view! { ... })` / `None`
- Iteration: `.into_iter().map(|item| view! { ... }).collect::<Vec<_>>()`
- Pre-rendered HTML strings: `<div inner_html={html_string}/>` or `<span inner_html={html_string}/>`
- All self-closing tags must use `/>` syntax (Leptos requirement): `<input />`, `<meta />`, `<br />`

### When to use `view!` vs raw `format!()`

Use the `view!` macro for top-level page bodies and standard page sections. Use raw `format!()` string building for complex, data-driven table rendering where the HTML structure is deeply conditional or involves nested iteration (e.g., message blocks, SSE event tables, tool parameter tables). Functions that return raw HTML strings are injected via `<div inner_html={html_string}/>`.

## CSS

Single embedded `<style>` block in `page_layout()` (`server/src/pages/mod.rs`). **No external CSS, no inline styles.** Monospace font, table-based layout, minimal borders.

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

Add new CSS classes to the `page_layout()` style block in `server/src/pages/mod.rs`. Never use inline styles. Never add external CSS.

## Shared helpers (`server/src/pages/mod.rs`)

| Helper | Purpose |
|--------|---------|
| `page_layout(title, body_html)` | Wraps body in full HTML document with `<style>` block |
| `html_escape(s)` | Escapes `& < > "` for use in raw HTML strings |
| `collapsible_block(content, css_class)` | Show-more/show-less for content > 200 chars |

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
| `breadcrumb_html(session, req, current_page)` | Generates `<h1>` breadcrumb for request detail/subpages |
| `render_kv_table(json_str)` | Renders a JSON object as a Key/Value table with collapsible values |
| `render_response_headers(req)` | Renders response status + headers as Key/Value table |

## Page structure pattern

Each page follows this standard section order (all sections optional except breadcrumb):

1. `<h1>` breadcrumb — always present
2. `<h2>"Navigation"</h2>` — action links (Edit, New, Back)
3. `<h2>"Info"</h2>` — key-value detail table
4. `<h2>"{Content}"</h2>` — list/content sections with total count
5. `<h2>"Subpages"</h2>` — Page/Count table linking to child pages
6. `<h2>"Actions"</h2>` — optional conditional actions (e.g., "Activate" button)

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

**Index page**: breadcrumb, Navigation ("New {Item}", "Back"), item list table with total count.

**Detail page**: breadcrumb, Navigation ("Edit {Item}", "Back"), Info table (key-value rows), Subpages table (Page/Count links to child pages). May include an optional Actions section after Subpages.

**Edit page**: breadcrumb (parent is a link), Navigation ("Back"), form with current values pre-filled, "Save" submit.

**Subpage list** (`/{sub}`): breadcrumb (parent detail page is a link), Navigation ("New {Item}", "Back"), items table with total count. Each row shows item fields, an "Edit" link to the edit page, and a Delete button. The add form is **not** on this page — it lives on the separate `/new` subpage.

**Subpage edit** (`/{sub}/{item_id}/edit`): breadcrumb (parent subpage list is a link), Navigation ("Back"), edit form with current values pre-filled, "Save" submit. POST submits to the same `/{sub}/{item_id}/edit` URL, which redirects back to the list after saving.

**Subpage add** (`/{sub}/new`): breadcrumb (parent subpage list is a link), Navigation ("Back"), add form, and optional suggested defaults. POST action submits to the parent subpage list URL (`/{sub}`), which redirects back to the list after creating.

**Content subpage**: breadcrumb, Navigation ("Back"), total count (when applicable), content section. These are leaf pages with no Subpages table.

### Breadcrumb (`<h1>`)

The `<h1>` is a breadcrumb trail. All ancestor pages are `<a>` links; the current (terminal) page is plain text. Separated by `" / "`.

```rust
// Top-level page (one level below Home):
<h1>
    <a href="/_dashboard">"Home"</a>
    " / "
    "Items"
</h1>

// Nested page:
<h1>
    <a href="/_dashboard">"Home"</a>
    " / "
    <a href="/_dashboard/items">"Items"</a>
    " / "
    {format!("Item {}", item.name)}
</h1>

// Deeply nested page:
<h1>
    <a href="/_dashboard">"Home"</a>
    " / "
    <a href="/_dashboard/items">"Items"</a>
    " / "
    <a href={format!("/_dashboard/items/{}", item.id)}>{format!("Item {}", item.name)}</a>
    " / "
    "Children"
</h1>
```

The Home page itself just uses `<h1>"Home"</h1>` with no links.

For detail pages (under `server/src/pages/detail/`), use the shared `breadcrumb_html()` helper from `server/src/pages/detail/common.rs` to generate the breadcrumb consistently.

### Navigation section (`<h2>"Navigation"</h2>`)

Immediately after the breadcrumb, add a Navigation heading with action links in a single-column table:

```rust
<h2>"Navigation"</h2>
<table>
    <tr><td><a href="/_dashboard/{entity}/new">"New {Item}"</a></td></tr>
    <tr><td><a href="javascript:history.back()">"Back"</a></td></tr>
</table>
```

Common navigation actions:

- **Create links** — e.g. `"New Item"` pointing to a `/new` route
- **Edit links** — e.g. `"Edit Item"` pointing to an `/edit` route
- **Related page links** — links to sibling or related sections
- **"Back"** — `<a href="javascript:history.back()">` — present on every page except Home, always the **last** link in the table

The Home page has no Navigation section — it only has Subpages.

### Subpages table

The Subpages section uses a two-column table with "Page" and "Count" headers. Count is the number of child items (or empty when not applicable):

```rust
<h2>"Subpages"</h2>
<table>
    <tr>
        <th>"Page"</th>
        <th>"Count"</th>
    </tr>
    <tr>
        <td><a href={sub_href}>"Child Items"</a></td>
        <td>{child_count}</td>
    </tr>
</table>
```

### Total count on list sections

Every section that displays a list of items shows a total count immediately after the `<h2>` heading:

```rust
<h2>"Items"</h2>
<p>{format!("Total: {}", items.len())}</p>
{if items.is_empty() {
    Either::Left(view! { <p>"No items yet."</p> })
} else {
    Either::Right(view! { <table>...</table> })
}}
```

This applies to all list pages (index pages, subpage item tables) and content subpages.

### Actions section (optional)

Conditionally rendered after Subpages. Uses `Either` to show/hide. Contains POST forms for state-changing actions:

```rust
{if show_action {
    Either::Left(view! {
        <h2>"Actions"</h2>
        <form method="POST" action={action_url}>
            <button type="submit">"Action Label"</button>
        </form>
    })
} else {
    Either::Right(view! {})
}}
```

## Tables

Tables are the primary layout primitive. Types:

### Key-value info table (no headers)

```rust
<table>
    <tr><td>"Label"</td><td>{value}</td></tr>
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

Tables can be nested for structured data (e.g., tool parameters inside a tools table, key-value pairs inside message cells). No special CSS needed — nested tables inherit the same styles.

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
- Checkboxes use `value="1"` and `checked={bool_value}` for edit forms
- Placeholders only where they hint at expected format (e.g. URLs: `placeholder="https://api.example.com"`, auth tokens: `placeholder="Bearer sk-..."`). Do not add placeholders on simple name or pattern fields
- Delete and edit actions always use dedicated routes (`/{item_id}/delete`, `/{item_id}/edit`), never query parameters
- Submit labels: "Create" for new items, "Save" for edits, "Add Filter" / "Add" for sub-items
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
    Either::Right(view! {})
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

Injected via `<span inner_html={copy_link_html(&url)}/>` after the displayed URL text.

### 2. Back navigation

`<a href="javascript:history.back()">"Back"</a>` — used in the Navigation section on every page except Home.

## Auto-refresh

Pages that monitor live data use a meta refresh tag (3-second interval). Controlled by a query parameter toggle link:

```rust
// Meta tag (inside view! body, before other elements)
{if auto_refresh {
    Some(view! { <meta http-equiv="refresh" content="3"/> })
} else {
    None
}}

// Toggle link (near the content section)
<a href={refresh_toggle_href}>{refresh_toggle_label}</a>
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

## Filtered rows and badges

For items that match a filter (tool names, system prompt patterns), apply visual dimming:

### Row-level filtering (in raw HTML strings)

```rust
let filtered = filters.iter().any(|f| f == name);
let row_class = if filtered { " class=\"filtered-row\"" } else { "" };

// Name cell with badge
let name_html = if filtered {
    format!(
        "{} <span class=\"filtered-badge\">[FILTERED]</span>",
        html_escape(name)
    )
} else {
    html_escape(name)
};

html.push_str(&format!("<tr{}><td>{}</td>...</tr>", row_class, name_html));
```

### Regex-based filter matching

System filters support regex patterns with plain-string fallback:

```rust
fn matched_filter<'a>(text: &str, filters: &'a [String]) -> Option<&'a str> {
    filters.iter().find_map(|f| {
        let matched = match Regex::new(f) {
            Ok(re) => re.is_match(text),
            Err(_) => text.contains(f.as_str()),
        };
        if matched { Some(f.as_str()) } else { None }
    })
}
```

## JSON and code display

Three rendering strategies based on data shape:

| Strategy | When to use | Implementation |
|----------|-------------|----------------|
| Key-value table | JSON objects | `render_kv_table()` — `<table>` with Key/Value headers, values use `collapsible_block()` |
| Pre-formatted text | Raw/unparseable JSON, small blocks | `<pre>{html_escape(json)}</pre>` |
| Readonly textarea | Full request JSON | `<textarea readonly rows="30" cols="80" wrap="off">{html_escape(json)}</textarea>` |
| Collapsible block | Long text in table cells | `collapsible_block(text, "")` (200-char threshold) |

### SSE raw data pattern

For collapsible raw JSON in SSE event tables, use a minimal `<details>` with "show raw" label:

```rust
let raw_html = format!(
    r#"<details class="collapsible"><summary><span class="show-more">show raw</span></summary><pre class="collapsible-full">{}</pre></details>"#,
    html_escape(&formatted_json),
);
```

## Text preview truncation

When showing previews of long content in list tables:

| Context | Max chars | Format |
|---------|-----------|--------|
| Last user message | 80 | `"{text}..."` |
| Tool use params | 40 | `"tool_use({name}): {params}..."` |
| Tool result | 60 | `"tool_result: {text}..."` |
| Thinking preview | 40 | `"thinking: {text}..."` |
| Generic text block | 60 | `"{text}..."` |

Newlines in previews are replaced with spaces: `text.replace('\n', " ")`.

## Cache control labels

Message blocks with `cache_control` field show an inline label:

```rust
fn cache_control_label(block: &serde_json::Value) -> String {
    block
        .get("cache_control")
        .and_then(|c| c.get("type"))
        .and_then(|t| t.as_str())
        .map(|t| format!(" (cache: {})", html_escape(t)))
        .unwrap_or_default()
}
```

Appended to the block type cell: `"text (cache: ephemeral)"`.

## HTML escaping

Always escape dynamic content in raw HTML strings using `html_escape()`. The function escapes `& < > "`. This is **not** needed inside `view!` macro text nodes (Leptos handles escaping), but **is** needed in:

- `format!()` strings that produce HTML
- Content passed to `inner_html`
- Arguments to `collapsible_block()`'s return value (it escapes internally)

## Status indicators

Simple text strings, no special styling:
- Active/inactive states: `"active"` / `"inactive"` / `"--"`
- Timestamps: Displayed as-is from the database (ISO 8601 format), no client-side formatting
- Empty/missing values: `String::new()` or `.unwrap_or_default()`
