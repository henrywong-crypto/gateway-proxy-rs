# CLAUDE.md

## Code Conventions

### Imports

Always import items at the top of the file with `use` statements instead of using fully qualified paths inline:

```rust
// Good
use foo::bar::{baz, Qux};
let x: Qux = baz();

// Bad
let x: foo::bar::Qux = foo::bar::baz();
```

Exceptions — these are fine to use inline without a `use` import:

- `serde_json::to_string`, `serde_json::from_slice`, `serde_json::from_str`, `serde_json::to_vec`
- `serde_json::Value`
- `serde_json::json!`
- `tracing_subscriber::fmt::init()`
- `std::env::var`

Combine `use` statements that share the same top-level crate into a single `use` with nested paths:

```rust
// Good
use hyper::{
    body::Bytes,
    rt::{Read, Write},
    Uri,
};

// Bad
use hyper::body::Bytes;
use hyper::rt::{Read, Write};
use hyper::Uri;
```

Group imports into two blocks separated by one blank line:

1. **External** — `std`, third-party crates, workspace crates, `self::`, `super::` (no blank lines within this group)
2. **Crate-local** — everything starting with `crate::` (no blank lines within this group)

If only one group exists, there are no blank lines in the import section.

```rust
// Good
use std::collections::HashMap;
use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;

use crate::pages;
use crate::Args;

// Bad — extra blank lines within the first group
use std::collections::HashMap;

use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;

use crate::pages;
use crate::Args;
```

### Versioning

All crate versions use 3-part semver (e.g. `0.1.0`).
