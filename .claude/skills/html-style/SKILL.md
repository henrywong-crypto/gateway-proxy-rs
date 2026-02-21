---
name: html-style
description: HTML/CSS conventions for this repo's server-rendered dashboard pages. Use when writing or modifying pages in server/src/pages/.
user-invocable: false
---

# HTML style conventions

## Framework

Pages use **Leptos SSR** (`view!` macro) — JSX-like syntax compiled to HTML strings on the server.

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
- Iteration: `.into_iter().map(|item| view! { ... }).collect::<Vec<_>>()`
- Pre-rendered HTML strings: `<div inner_html={html_string}/>`

## CSS

Single embedded `<style>` block in `page_layout()` (`server/src/pages/mod.rs`). No external CSS, no JS. Monospace font, table-based layout, minimal borders. Avoid inline styles.

Add new classes to the `page_layout()` style block rather than adding inline styles.

## Shared helpers (pages/mod.rs)

- `page_layout(title, body_html)` — wraps body in full HTML document
- `html_escape(s)` — escapes `& < > "` for use in raw HTML strings
- `collapsible_block(content, css_class)` — show-more/show-less for content > 200 chars

## Page structure pattern

Each page follows this layout:

1. `<h1>` breadcrumb: `<a href="/_dashboard">"Home"</a> " / " "Current Page"`
2. `<h2>"Navigation"</h2>` with link table (Back, related pages)
3. `<h2>"Section"</h2>` with content tables or forms

## Forms

- `method="POST"` with `action` URL
- Inputs inside `<table>` rows: `<td><label>` / `<td><input>`
- Buttons via `<input type="submit">` or `<button type="submit">`

## No JavaScript

All interactivity is server-side (HTML forms + redirects). Auto-refresh uses `<meta http-equiv="refresh" content="3"/>`.
