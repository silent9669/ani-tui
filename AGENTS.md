# AGENTS.md - Coding Guidelines for ani-tui

## Build, Lint, and Test Commands

```bash
# Build the project
cargo build --release

# Run all tests
cargo test --release

# Run a specific test by name
cargo test --release test_decode_provider_id

# Run tests in a specific module
cargo test --release providers::allanime::tests

# Check formatting
cargo fmt --all -- --check

# Apply formatting
cargo fmt --all

# Run Clippy lints (treats warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings

# Run security audit
cargo audit
```

## Code Style Guidelines

### Imports
- Group imports: std lib first, then external crates, then local modules
- Use `use super::*` in test modules
- Prefer `anyhow::Result` for error handling

```rust
// Standard library
use std::collections::HashMap;
use std::time::Duration;

// External crates
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{self, HeaderMap};

// Local modules
use crate::config::Config;
```

### Formatting
- Use `cargo fmt` with default settings (no custom rustfmt.toml)
- Max line length: 100 characters (soft limit)
- Use 4 spaces for indentation

### Naming Conventions
- **Types**: PascalCase (`AnimeProvider`, `StreamInfo`)
- **Functions/Variables**: snake_case (`get_stream_url`, `episode_number`)
- **Constants**: SCREAMING_SNAKE_CASE (`ALLANIME_API`, `REQUEST_TIMEOUT`)
- **Traits**: PascalCase with descriptive names (`AnimeProvider`)
- **Error types**: End with "Error" (`AniTuiError`)

### Types
- Use strong typing with structs/enums over primitive types
- Derive common traits: `Debug`, `Clone`, `Serialize`, `Deserialize`
- Use `Option<T>` for nullable fields
- Use `Result<T>` (anyhow) for fallible operations

### Error Handling
- Use `anyhow` for application errors with `?` operator
- Use `thiserror` for custom error enums
- Provide context with `.with_context()`

```rust
// Good
let conn = Connection::open(&db_path)
    .with_context(|| format!("Failed to open database at {:?}", db_path))?;

// Custom errors
#[derive(Error, Debug)]
pub enum AniTuiError {
    #[error("Provider error: {0}")]
    Provider(String),
}
```

### Async/Await
- Use `tokio` runtime with `#[tokio::main]`
- Mark trait methods with `#[async_trait]`
- Use `Arc<Mutex<T>>` for shared state (tokio::sync::Mutex)

### Testing
- Unit tests in `mod tests` at bottom of file
- Use `#[tokio::test]` for async tests
- Mark broken tests with `#[ignore = "reason"]`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_provider_id() {
        // test code
    }

    #[tokio::test]
    #[ignore = "API is unavailable"]
    async fn test_api_call() {
        // async test
    }
}
```

### Project Structure
```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Public module exports
├── error.rs          # Error types
├── config.rs         # Configuration
├── db.rs             # Database layer
├── player.rs         # Video player integration
├── image_manager.rs  # Image caching
├── providers/        # Anime source providers
│   ├── mod.rs        # Provider trait + registry
│   ├── allanime.rs
│   └── kkphim.rs
├── ui/               # TUI components
│   ├── mod.rs
│   ├── app.rs        # Main app loop
│   └── components/
└── image/            # Image processing
```

### Clippy Exceptions
When necessary, allow specific lints at file level:

```rust
#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]
```

## CI/CD Requirements
All PRs must pass:
1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --release` (Ubuntu, macOS, Windows)
4. `cargo audit` (security check)

## Dependencies
- **Async**: tokio
- **HTTP**: reqwest
- **TUI**: ratatui, crossterm
- **Serialization**: serde, serde_json, toml
- **Database**: rusqlite
- **Errors**: anyhow, thiserror
- **CLI**: clap
