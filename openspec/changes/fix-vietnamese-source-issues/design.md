## Context

The ani-tui application currently has several critical bugs affecting the Vietnamese anime source (KKPhim) and image rendering:

1. **Source Selection**: Currently allows multiple sources (checkboxes), but this causes confusion and makes it unclear which source results are showing. Users should select EN **OR** VN exclusively.

2. **Image Preview**: Chafa is installed but images display as black squares due to incorrect terminal size calculations and rendering logic.

3. **KKPhim Episode Count**: The API returns incorrect episode counts (showing 1 episode instead of 500+ for series like Naruto Shippuden). This is likely due to parsing the wrong field from the API response.

4. **Vietnamese Metadata**: AniList searches for Vietnamese titles fail to return metadata, showing "Loading metadata..." indefinitely. This is likely due to title matching issues or missing metadata enrichment calls.

5. **Vietnamese Streaming**: Player fails to open for Vietnamese episodes. This could be stream URL format issues, API endpoint changes, or response parsing errors.

## Goals / Non-Goals

**Goals:**
- Implement single source selection (radio button style) to prevent confusion
- Fix chafa image rendering to display actual cover images
- Debug and fix KKPhim episode fetching to return correct counts
- Ensure AniList metadata loads for Vietnamese anime titles
- Fix stream URL fetching so Vietnamese episodes play correctly
- Add comprehensive error handling and logging

**Non-Goals:**
- Adding new anime sources beyond AllAnime and KKPhim
- Implementing video quality selection
- Adding download functionality
- Changing the UI layout significantly
- Supporting non-anime content

## Decisions

### Decision 1: Single Source Selection Implementation
**Choice**: Change from checkboxes to radio-button style selection in the SourceSelectModal.

**Rationale**: 
- Current multi-select causes confusion about which source's results are displayed
- The search already filters by enabled_sources, but having both active makes results mixed
- Single selection simplifies the mental model: pick one language, get results in that language

**Implementation**:
- Modify `SourceSelectModal` to track a single `selected_source` instead of a list
- When user toggles a source, uncheck the other
- Update `ProviderRegistry.search_filtered()` to only search the selected source
- Update UI to show radio button indicators (◉ / ○) instead of checkboxes

### Decision 2: Fix Chafa Image Rendering
**Choice**: Fix the terminal size calculations and use actual chafa rendering instead of placeholders.

**Rationale**:
- Chafa is already a dependency and installed
- The issue is incorrect width/height calculations causing rendering failures
- Fallback to placeholder only when chafa fails or is unavailable

**Implementation**:
- In `PreviewPanel.render()`, calculate proper dimensions:
  - Width: `area.width` (terminal columns)
  - Height: `area.height * 2` (terminal rows, accounting for character aspect ratio)
- Call `ChafaRenderer.render()` with calculated dimensions
- Handle errors gracefully with fallback to ASCII placeholder

### Decision 3: Debug KKPhim Episode API
**Choice**: Review and fix the episode parsing logic in `kkphim.rs`.

**Rationale**:
- KKPhim API returns nested JSON structures
- Current parsing likely extracts the wrong field (possibly getting season count instead of episode count)
- Need to examine actual API response structure

**Implementation**:
- Add debug logging to `get_episodes()` to log raw API response
- Verify the JSON path: `data.item.episodes[].server_data[]`
- Ensure we're counting all episodes across all servers
- Test with known series: Naruto Shippuden (should be 500+), seasonal anime (typically 12-24)

### Decision 4: Fix Vietnamese Metadata Loading
**Choice**: Ensure metadata is fetched when navigating to Vietnamese anime results.

**Rationale**:
- Metadata loading works for English sources but not Vietnamese
- Likely cause: `load_preview()` not being called, or AniList search failing for Vietnamese titles
- Need to ensure metadata enrichment happens regardless of source

**Implementation**:
- In `app.rs`, ensure `load_preview()` is called when navigating search results (Up/Down arrows)
- Debug `MetadataCache.search_and_cache()` to verify it's searching AniList
- If AniList fails to match Vietnamese titles, consider using the original title or adding romanized variants
- Add retry logic or better error handling for metadata fetch failures

### Decision 5: Fix Vietnamese Stream URL Fetching
**Choice**: Debug the stream URL generation for KKPhim episodes.

**Rationale**:
- Stream URLs work for AllAnime but not KKPhim
- Likely causes: wrong episode ID format, API endpoint issues, or response parsing
- Need to verify the complete flow from episode selection to stream URL

**Implementation**:
- Add logging to `select_anime()` and `play_current_episode()`
- Verify episode ID format: should be `{anime_slug}:{episode_number}`
- Debug `kkphim.get_stream_url()` to verify:
  - API endpoint is correct
  - Episode lookup finds the right episode
  - Stream URL is extracted from the correct JSON field
- Test with actual API calls to ensure m3u8 URLs are returned

### Decision 6: Comprehensive Error Handling
**Choice**: Add detailed error logging and user feedback throughout the flow.

**Rationale**:
- Current errors are silent or show generic messages
- Need visibility into what's failing for debugging
- Users should know if an episode failed to load or if there's no stream available

**Implementation**:
- Add `tracing::info!()` and `tracing::error!()` calls at key points:
  - Source selection changes
  - Search execution with result counts per source
  - Episode loading with count
  - Stream URL fetch attempts and results
- Show user-friendly toast messages for failures
- Ensure errors don't crash the app

## Risks / Trade-offs

**[Risk] Breaking existing English source functionality**
→ Mitigation: Test both sources thoroughly after changes. Keep AllAnime implementation mostly untouched.

**[Risk] KKPhim API changes or rate limiting**
→ Mitigation: Add proper error handling. Consider adding request throttling if needed.

**[Risk] AniList API rate limits for metadata**
→ Mitigation: Already has 7-day caching. Ensure cache is working properly.

**[Risk] Chafa rendering performance issues on large images**
→ Mitigation: Limit image size in terminal. Use caching to avoid re-rendering.

**[Trade-off] Single source selection reduces flexibility**
→ Rationale: While users can't see both EN and VN results simultaneously, the experience is clearer and less confusing.

## Migration Plan

**Deployment Steps:**
1. Implement single source selection
2. Fix image rendering
3. Debug and fix KKPhim episode count
4. Fix Vietnamese metadata loading
5. Fix Vietnamese streaming
6. Add comprehensive logging
7. Test both sources end-to-end
8. Update documentation

**Rollback Strategy:**
- All changes are additive or fix bugs
- No database schema changes required
- Can rollback by reverting git commits

## Open Questions

1. **KKPhim API Structure**: Need to examine actual API response to find correct episode count field
2. **AniList Matching**: May need to improve title matching for Vietnamese anime (some Vietnamese titles may not match AniList entries)
3. **Stream URL Format**: Need to verify if KKPhim returns m3u8 or other stream formats
4. **Performance**: Should we add loading indicators for slow operations (image download, metadata fetch)?