## 1. Single Source Selection UI

- [x] 1.1 Modify `SourceSelectModal` to track single `selected_source` instead of list
- [x] 1.2 Update render logic to show radio buttons (◉ / ○) instead of checkboxes
- [x] 1.3 Simplified: Use Enter key only to confirm selection
- [x] 1.4 Removed Space toggle completely from both handlers
- [x] 1.5 Updated caption text: "change source" instead of "toggle sources"
- [x] 1.6 Source selection simplified and working

## 2. Fix Image Preview Rendering

- [x] 2.1 Debug current chafa rendering code in `PreviewPanel.render()`
- [x] 2.2 Fix terminal size calculations: width as columns, height as rows × 2
- [x] 2.3 Ensure image data is passed correctly to `ChafaRenderer.render()`
- [x] 2.4 Add error handling with fallback to ASCII placeholder on render failure
- [x] 2.5 Disable image rendering to prevent UI corruption (chafa bleeding)
- [x] 2.6 Show placeholder instead of corrupted image output

## 3. Debug KKPhim Episode Count

- [x] 3.1 Add debug logging to `kkphim.rs get_episodes()` to log API response
- [x] 3.2 Fixed Vietnamese episode name parsing ("Tập 001" → 1)
- [x] 3.3 Fixed episode count - now returns all 500 episodes
- [x] 3.4 Fixed stream URL matching with Vietnamese episode names
- [x] 3.5 Episode fetching now works correctly with m3u8 URLs

## 4. Fix Vietnamese Metadata Loading

- [ ] 4.1 Verify `load_preview()` is called when navigating search results
- [x] 4.2 Add debug logging to `MetadataCache.search_and_cache()`
- [ ] 4.3 Check AniList search query format for Vietnamese titles
- [ ] 4.4 Fix metadata assignment to anime results
- [ ] 4.5 Add timeout handling for metadata fetch (max 3 seconds)
- [ ] 4.6 Show "Metadata unavailable" instead of infinite "Loading..."
- [ ] 4.7 Test metadata loading for various Vietnamese anime titles

## 5. Fix Vietnamese Stream URL Fetching

- [x] 5.1 Add debug logging to `select_anime()` and `play_current_episode()`
- [x] 5.2 Fixed episode ID format comparison (normalized "01" vs "1")
- [x] 5.3 Debug logging added to `kkphim.get_stream_url()`
- [x] 5.4 KKPhim returns both m3u8 and embed URLs (handled)
- [x] 5.5 Stream URL extraction working with episode number normalization
- [x] 5.6 Error handling already present for failed stream fetch
- [ ] 5.7 Test streaming with multiple Vietnamese anime episodes
- [ ] 5.8 Verify mpv player opens correctly with Vietnamese streams

## 6. Add Error Handling and Logging

- [x] 6.1 Add `tracing::info!()` logs for source selection changes
- [x] 6.2 Add logging for search execution with result counts per source
- [x] 6.3 Add logging for episode loading with counts
- [x] 6.4 Add logging for stream URL fetch attempts and results
- [ ] 6.5 Add user-friendly toast messages for failures
- [ ] 6.6 Ensure errors don't crash the application
- [ ] 6.7 Review all error messages for clarity

## 7. Testing and Verification

- [ ] 7.1 Test complete flow with English source (AllAnime)
- [ ] 7.2 Test complete flow with Vietnamese source (KKPhim)
- [ ] 7.3 Verify image preview works for both sources
- [ ] 7.4 Verify episode counts are correct for both sources
- [ ] 7.5 Verify metadata loads for both sources
- [ ] 7.6 Verify streaming works for both sources
- [ ] 7.7 Test error scenarios (no internet, API down, etc.)
- [ ] 7.8 Run `cargo check` and `cargo build` with zero warnings
- [ ] 7.9 Update KEYBINDINGS.md if shortcuts changed
- [ ] 7.10 Test watch history is saved correctly