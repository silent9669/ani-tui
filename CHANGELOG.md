# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2025-01-30

### Added
- **Dual-Source Search**: Search both AllAnime (English) and KKPhim (Vietnamese) simultaneously
- **Language Badges**: [EN] and [VN] badges in search results
- **AniList Integration**: Fetch metadata (ratings, genres, descriptions) from AniList
- **Metadata Caching**: 7-day TTL cache for AniList data in SQLite
- **Image Pipeline**: Parallel image downloading (10 concurrent downloads)
- **Chafa Renderer**: Terminal image display using chafa
- **Memory Cache**: LRU cache for 50 images in memory
- **Splash Screen**: Animated startup screen
- **Source Selection**: Toggle between English and Vietnamese sources
- **Search Overlay**: Netflix-style search with Shift+S
- **Preview Panel**: Live anime details panel in search results
- **Player Controls**: Inline control menu (Next/Prev/Episodes/Favorite/Back)
- **End Screen**: Post-playback options (Next/Replay/Back)
- **Episode List Modal**: Choose any episode while watching
- **Favorites**: Save anime to favorites list
- **Cross-Platform Packaging**:
  - Homebrew formula for macOS
  - Scoop manifest for Windows
  - PowerShell one-liner installer

### Changed
- Updated ProviderRegistry to support multiple providers
- Enhanced dashboard with "Continue Watching" section
- Improved search with auto-complete and debouncing
- Updated config to support dual source selection

### Fixed
- Provider API integration issues
- Image loading performance

## [0.1.0] - 2024-XX-XX

### Added
- Initial release with basic TUI
- Single provider support (Gogoanime)
- Basic search and episode selection
- Watch history tracking
- Configuration file support
- Database layer with SQLite