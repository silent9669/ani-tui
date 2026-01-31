## Why

The Vietnamese anime source (KKPhim) has critical bugs preventing proper usage - episode counts are incorrect (showing 1 episode instead of 500+), metadata is not loading (showing "Loading metadata..." indefinitely), and the player fails to open after episode selection. Meanwhile, the English source (AllAnime) works perfectly. Additionally, image previews using chafa are displaying as black squares instead of actual images. These issues make the Vietnamese source unusable and degrade the overall user experience.

## What Changes

- **BREAKING**: Change source selection from multi-select (checkboxes) to single-select (radio buttons) - users can only choose EN **OR** VN, not both simultaneously
- Fix image preview rendering with chafa to display actual anime cover images instead of black squares
- Debug and fix KKPhim episode fetching API to return correct episode counts (e.g., Naruto Shippuden should show 500+ episodes, not 1)
- Fix AniList metadata search for Vietnamese anime titles to load descriptions, ratings, and genres
- Debug Vietnamese stream URL fetching to ensure player opens correctly after episode selection
- Add proper error handling and logging for Vietnamese source operations

## Capabilities

### New Capabilities
- `single-source-selection`: Allow users to select only one source at a time (EN or VN) using radio button interface instead of checkboxes
- `fix-image-preview`: Proper chafa integration for displaying anime cover images in the preview panel with correct sizing and color rendering
- `kkphim-episode-fix`: Correct episode count fetching from KKPhim API by debugging the episode list endpoint and parsing logic
- `vietnamese-metadata`: Fix AniList metadata loading for Vietnamese anime titles to properly match and fetch descriptions, ratings, and genres
- `vietnamese-streaming`: Debug and fix stream URL generation for Vietnamese sources to ensure mpv player opens with valid URLs

### Modified Capabilities
- *(none - this is primarily bug fixes and UI improvements)*

## Impact

**Affected Code:**
- `src/ui/modern_components.rs` - Source selection modal, preview panel image rendering
- `src/ui/app.rs` - Source selection logic, episode selection screen
- `src/providers/kkphim.rs` - Episode fetching and stream URL generation
- `src/metadata/mod.rs` - AniList metadata search for Vietnamese titles
- `src/providers/mod.rs` - Provider registry source filtering

**Affected User Experience:**
- Source selection screen will only allow one source at a time
- Vietnamese anime will display correct episode counts
- Image previews will render properly in the preview panel
- Vietnamese anime metadata (description, rating, genres) will load correctly
- Player will open successfully for Vietnamese episodes

**Dependencies:**
- chafa (for image rendering) - already required but needs proper integration
- KKPhim API - existing dependency but needs debugging
- AniList API - existing dependency but needs better matching for Vietnamese titles