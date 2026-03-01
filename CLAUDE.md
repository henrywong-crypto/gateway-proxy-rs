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

### Function Naming

Start every function name with a verb. The nouns in the name must match the type being returned or acted on.

```rust
// Good — verb first, noun matches return type
fn get_animal(id: &str) -> Option<Animal>;
fn list_wild_animals(region: &str) -> Vec<WildAnimal>;
fn count_animals() -> i64;
fn create_animal(params: &AnimalParams) -> Animal;
fn update_animal(id: &str, params: &AnimalParams) -> Result<()>;
fn delete_animal(id: &str) -> Result<()>;
fn clear_animals() -> Result<()>;

// Good — single-field setter names the entity and field
fn set_animal_name(id: &str, name: &str) -> Result<()>;
fn set_cage_temperature(id: &str, temp: f64) -> Result<()>;

// Good — transform / produce / convert
fn build_feed_schedule(animals: &[Animal]) -> FeedSchedule;
fn parse_tag_number(raw: &str) -> Option<TagNumber>;
fn validate_cage_size(cage: &Cage) -> Result<(), CageError>;
fn encode_payload(data: &Payload) -> Vec<u8>;
fn decode_payload(raw: &[u8]) -> Result<Payload>;
fn extract_metadata(raw: &[u8]) -> Metadata;
fn compute_feed_cost(schedule: &FeedSchedule) -> f64;
fn format_animal_report(animal: &Animal) -> String;
fn render_animals_view(animals: &[Animal]) -> String;
fn render_new_animal_form(species: &[Species]) -> String;

// Bad — noun doesn't match return type
fn list_animals() -> Vec<WildAnimal>;  // returns WildAnimal, not Animal
fn get_cage(id: &str) -> Option<CageStatus>;  // returns CageStatus, not Cage

// Bad — missing verb
fn animals(region: &str) -> Vec<Animal>;
fn animal_name(id: &str) -> String;

// Bad — ambiguous setter (which field?)
fn set_animal(id: &str, name: &str) -> Result<()>;  // use set_animal_name
```

### Versioning

All crate versions use 3-part semver (e.g. `0.1.0`).
