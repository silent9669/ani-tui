# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.8.2] - 2026-04-27
- **AllAnime Decryption Fix**: Implemented a GraphQL GET request bypass using persisted queries to circumvent Cloudflare TLS fingerprinting issues.
- **Homebrew Detection Refinement**: Improved `is_homebrew_install` logic to provide more accurate installation detection on macOS and avoid false positives in development environments.
- **Project Configuration**: Cleaned up `.gitignore` and consolidated build settings.

## [3.8.1] - 2026-04-23
### Fixed
- Updated AllAnime `tobeparsed` decryption key and byte offsets for `_m: b7`.
- Mapped KKPhim and OPhim movie entries named `Full` to episode 1.

## [3.8.0] - 2026-04-19
### Added
- Global `Shift+R` shortcut to access activity logs for easier debugging.

### Improved
- **Flicker-Free Image Rendering**: Optimized iTerm2/Warp protocol with hash-based change detection.
- Unified provider registry with stable English and Vietnamese sources.

### Removed
- AniWatch source.
- Unstable NguonC (VN) source.

## [3.7.9] - 2026-04-18

### Added
- **Real-time Log Tailing**: Optimized the Report screen to efficiently display the last 500 lines of system and player activity.
- **AES Decryption for AllAnime**: Restored AllAnime playback by implementing AES-256-CTR decryption for `tobeparsed` GraphQL responses.
- **Smart Windows Installer**: New Rust-based installer for automated environment setup on Windows.

### Fixed
- **Vietnamese Source Stability**: Fixed 403 "hmmm!" errors on KKPhim and OPhim by optimizing Referer and User-Agent headers.
- **Player Log Flushing**: Ensured `mpv` logs are correctly appended and flushed with high verbosity for the Report screen.
- **UI Performance**: Implemented 150ms selection debouncing and non-blocking asynchronous background searches.
- **Image Performance**: Optimized line-caching for Halfblock rendering on Windows and macOS Terminal.app.
- **Search Pagination**: Implemented 10 items per page with Left/Right or PgUp/PgDn navigation.

### Changed
- **Default Source Configuration**: Enabled AllAnime, KKPhim, and OPhim by default.
- **UI Aesthetics**: Standardized language flags (🇺🇸/🇻🇳); removed brackets and text labels for a cleaner, modern look.
- **Config Refactor**: Simplified `Config::load` and centralized source management.

## [3.7.8] - 2026-04-07

### Fixed
- Fixed AllAnime API by updating referrer URL to allmanga.to (matching ani-cli)

### Changed
- Improved search responsiveness by reducing debounce from 500ms to 200ms

## [3.7.7] - 2026-03-15

### Fixed
- Fixed partial image corruption on first dashboard load by detecting first render and forcing cache clear
- Fixed `is_first_render` detection in ImageRenderer to ensure clean terminal state on initial render

## [3.7.6] - 2026-03-15

### Fixed
- Fixed image rendering on first dashboard load - images now display correctly immediately on app startup
- Fixed state inconsistency between `current_image_data` and `current_anime_id` during initialization
