# Contributing to ani-tui

Thank you for your interest in contributing to ani-tui! This document provides guidelines and workflows for contributing.

## Table of Contents

- [Development Setup](#development-setup)
- [Image Rendering Changes](#image-rendering-changes)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)

## Development Setup

### Prerequisites

- Rust 1.70 or later
- chafa (for image rendering)
- mpv (for video playback)

### Building from Source

```bash
git clone https://github.com/silent9669/ani-tui
cd ani-tui
cargo build --release
```

### Running Tests

```bash
cargo test --release
```

## Image Rendering Changes

The image rendering system is complex and critical to the application. **All changes to image rendering must be documented.**

### Pre-Commit Hook

A pre-commit hook automatically detects changes to `src/ui/image_renderer.rs` and reminds you to update the documentation.

To install the hook:
```bash
# Use the versioned hooks directory
git config core.hooksPath .githooks
```

### Documentation Update Workflow

When you modify image rendering code:

1. **Make your changes** to `src/ui/image_renderer.rs`

2. **Run the documentation helper script:**
   ```bash
   ./scripts/update-image-docs.sh
   ```

3. **Follow the prompts** to add a changelog entry

4. **Stage the documentation changes:**
   ```bash
   git add docs/IMAGE_RENDERING.md
   git commit -m "fix(image): description of change"
   ```

### Changelog Format

See `docs/IMAGE_RENDERING.md` for the changelog template and examples.

## Testing with Daytona

We use Daytona sandboxes for multi-platform testing.

### Setup

1. **Install Daytona CLI:**
   ```bash
   brew install daytonaio/tap/daytona
   ```

2. **Configure API key:**
   ```bash
   export DAYTONA_API_KEY="your-api-key"
   ```

### Running Tests

```bash
# Test on Linux
./scripts/daytona-test.sh ani-tui-linux test

# Test on macOS Intel
./scripts/daytona-test.sh ani-tui-macos-intel test

# Test on macOS ARM
./scripts/daytona-test.sh ani-tui-macos-arm test

# Test on Windows
./scripts/daytona-test.sh ani-tui-windows test
```

## Pull Request Process

1. **Create a feature branch**
2. **Make your changes**
3. **Run quality checks:**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --release
   ```
4. **Update documentation** (especially for image rendering changes)
5. **Commit with conventional format:**
   - `feat: add new feature`
   - `fix(image): resolve flickering issue`
   - `docs: update changelog`
6. **Push and create PR**
7. **Ensure CI passes**

## Development Setup

### Prerequisites

- Rust 1.70 or later
- chafa (for image rendering)
- mpv (for video playback)

### Building from Source

```bash
git clone https://github.com/yourusername/ani-tui
cd ani-tui
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Code Style

We use `rustfmt` and `clippy` to maintain code quality:

```bash
cargo fmt
cargo clippy -- -D warnings
```

## Project Structure

```
src/
├── main.rs              # Application entry point
├── config.rs            # Configuration management
├── db.rs                # Database layer (SQLite)
├── image/               # Image pipeline module
│   └── mod.rs
├── metadata/            # AniList metadata module
│   └── mod.rs
├── player.rs            # Media player integration
├── providers/           # Anime source providers
│   ├── mod.rs           # Provider trait and registry
│   ├── allanime.rs      # AllAnime (English)
│   ├── kkphim.rs        # KKPhim (Vietnamese)
│   ├── gogoanime.rs     # Gogoanime
│   └── prowlarr.rs      # Prowlarr integration
└── ui/                  # User interface
    ├── mod.rs
    ├── app.rs           # Main application logic
    ├── components.rs    # Reusable UI components
    ├── modern_components.rs  # New UI components
    ├── player_controller.rs  # Player control state machine
    └── screens.rs       # Screen definitions
```

## Adding a New Provider

To add support for a new anime source:

1. Create a new file in `src/providers/` (e.g., `src/providers/newsource.rs`)
2. Implement the `AnimeProvider` trait
3. Update `ProviderRegistry` in `src/providers/mod.rs`

Example provider implementation:

```rust
use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use anyhow::Result;
use async_trait::async_trait;

pub struct NewProvider {
    client: reqwest::Client,
}

impl NewProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AnimeProvider for NewProvider {
    fn name(&self) -> &str {
        "NewProvider"
    }

    fn language(&self) -> Language {
        Language::English  // or Language::Vietnamese
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        // Implement search logic
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        // Implement episode fetching
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        // Implement stream URL fetching
    }
}
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests and ensure code quality (`cargo test && cargo clippy`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### PR Guidelines

- Provide a clear description of the changes
- Reference any related issues
- Ensure all tests pass
- Update documentation if needed
- Add examples for new features

## Reporting Issues

When reporting issues, please include:

- Operating system and version
- Rust version (`rustc --version`)
- ani-tui version
- Steps to reproduce
- Expected behavior
- Actual behavior
- Any error messages or logs

## Code of Conduct

This project follows the Rust Code of Conduct:

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Respect different viewpoints and experiences

## License

By contributing to ani-tui, you agree that your contributions will be licensed under the MIT License.