# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.7.6] - 2026-03-15

### Fixed
- Fixed image rendering on first dashboard load - images now display correctly immediately on app startup
- Fixed state inconsistency between `current_image_data` and `current_anime_id` during initialization

## [3.7.5] - 2026-03-14

### Added
- Auto-update feature with background version checking during splash screen
- `--update` CLI flag for self-updating the app
- `--check-update` CLI flag for manual update checking
- Persistent update notifications via SQLite database
- Cross-platform update support (macOS via Homebrew, Windows via Scoop, binary fallback)

### Changed
- Update check runs in background during splash screen with 3-second timeout
- Non-blocking update notifications that persist across app restarts

## [3.7.4] - 2026-03-09

### Added
- Homebrew tap auto-update via GitHub Actions
- Cross-platform release automation (macOS, Windows, Linux)

### Fixed
- Updated HOMEBREW_TAP_TOKEN secret for homebrew-tap repository access
- Fixed authentication issues with GitHub Actions workflow
- Fixed Windows build archive creation (added `-Force` flag)
- Fixed Homebrew formula template for brewbot compliance
- Fixed Ruby syntax check in formula validation
- Fixed double-v bug in Homebrew URL generation
- Changed `brew audit` to `ruby -c` (brew audit [path] disabled in CI)
- Added Homebrew tap setup with symlink creation for formula testing

### Technical
- CI/CD pipeline now fully automated for releases
- Homebrew formula automatically generated and committed to tap
- All builds (x86_64, aarch64, Windows, Linux) complete successfully

## [3.7.3] - 2026-03-09

### Fixed
- Homebrew formula template updated for brewbot compliance
- Simplified formula description and test block
- Removed livecheck block (using default GitHub strategy)

## [3.6.1] - 2026-03-08

### Added
- New Source Select screen (Shift+S from Home) for choosing between English and Vietnamese sources
- Background data loading during intro screen with progress bar
- Support for both AllAnime (English) and KKPhim (Vietnamese) sources
- Image preloading for smoother navigation in Continue Watching
- Episode grid view with pagination and filtering
- Player controls overlay with next/previous episode buttons

### Changed
- Improved intro screen with better ASCII art and progress bar
- Source selection workflow: Shift+S now opens dedicated source screen instead of modal
- Progress bar reaches 90% faster while maintaining 2-second intro duration
- Episode selection from player now uses full Episode Select screen instead of simple modal
- Continue Watching list now displays all anime (removed 5-item limit)

### Fixed
- Episode selection from media control now uses same UI as dashboard
- Source selection box no longer broken on first click from dashboard
- Images now clear properly when transitioning between screens
- Image preview reloads when returning to Search from Episode Select
- Watch history now refreshes when returning to Dashboard
- Source modal in Search page now works correctly (Shift+C)
- Fixed image rendering when returning to Dashboard from Episode Select
- Fixed Vietnamese source option not showing in source modal
- Fixed modal size being too large and overlaying content

### Known Issues
- **Watch history delay**: When watching anime from Search page and returning to Dashboard, the watched anime may not appear immediately in Continue Watching. Workaround: The list refreshes automatically after a few seconds, or restart the app.
- **Image rendering on Dashboard return**: When returning to Dashboard from Episode Select, the anime image may not render immediately. Workaround: Navigate to another anime and back, or the image will load after a brief delay.
- **Windows Terminal resize**: Resizing Windows Terminal while images are displayed may cause visual glitches. Workaround: Avoid resizing during image display, or press any key to refresh.

### Technical
- Added `needs_continue_watching_refresh` flag for dashboard refresh
- Added `needs_preview_load` flag for search preview refresh
- Improved image cache management and cleanup
- Fixed terminal graphics clearing logic to prevent flickering
- Optimized database queries for watch history

## [3.6.0] - 2026-03-01

### Added
- Initial support for Vietnamese subtitles via KKPhim provider
- Source selection modal (Shift+C in Search)
- Continue Watching section on Dashboard
- Image preview in search results
- Episode selection with grid layout
- Player control overlay

### Changed
- Complete UI redesign with Netflix-inspired layout
- Improved search with debounced input
- Better error handling and user feedback

### Fixed
- Various UI rendering issues
- Memory leaks in image loading
- Database connection handling

## [3.5.0] - 2026-02-15

### Added
- Initial release with AllAnime provider
- Basic search and episode selection
- Video playback via mpv
- Watch history tracking

### Features
- Terminal User Interface with ratatui
- Cross-platform support (macOS, Windows, Linux)
- Homebrew and Scoop package managers
- CI/CD with GitHub Actions

---

## Roadmap

### Planned for 3.7.0
- [ ] Fix watch history immediate update issue
- [ ] Fix image rendering on dashboard return
- [ ] Add keyboard shortcuts help screen
- [ ] Improve error messages for network failures
- [ ] Add configuration file support

### Future Ideas
- [ ] Download episodes for offline viewing
- [ ] Custom theme support
- [ ] Integration with more anime sources
- [ ] Subtitle customization options
- [ ] Watchlist / favorites feature

---

## Reporting Issues

If you encounter any issues not listed in Known Issues, please report them at:
https://github.com/silent9669/ani-tui/issues

When reporting, please include:
- Operating system and version
- Terminal application and version
- ani-tui version (`ani-tui --version`)
- Steps to reproduce the issue
- Any error messages
