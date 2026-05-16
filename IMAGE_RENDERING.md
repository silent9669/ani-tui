# Image Rendering System - Development History

## Overview

Cross-platform image preview for ani-tui using multiple terminal graphics protocols.

## Protocols Supported

- **Kitty Graphics Protocol** - Best quality, requires explicit clearing
- **iTerm2 Inline Images** - Widely supported (Warp, iTerm2, WezTerm), stateless
- **Sixel (via chafa)** - Optional for known sixel terminals
- **Halfblocks** - Stable in-buffer fallback for Windows Terminal and unknown terminals

## Key Design Decisions

### Protocol Detection Priority

1. Terminal.app → None (no support)
2. iTerm2 → iTerm2 protocol
3. Warp → iTerm2 protocol (avoids Kitty corruption issues)
4. WezTerm → Kitty protocol
5. Kitty → Kitty protocol
6. Windows Terminal → Halfblocks
7. Known sixel terminals with chafa → Sixel
8. Default → Halfblocks

Users can override detection with:

```bash
ANI_TUI_IMAGE_PROTOCOL=auto ani-tui
ANI_TUI_IMAGE_PROTOCOL=kitty ani-tui
ANI_TUI_IMAGE_PROTOCOL=iterm2 ani-tui
ANI_TUI_IMAGE_PROTOCOL=sixel ani-tui
ANI_TUI_IMAGE_PROTOCOL=halfblocks ani-tui
```

### Image Clearing Strategy

- **Kitty**: Use escape sequence `\x1b_Ga=d,d=i,i=1,q=2\x1b\\` to delete by ID
- **iTerm2**: Images drawn via OSC 1337 escape codes persist on screen
  - Solution: Track `last_rendered_area` and clear with spaces
- **Sixel**: Send string terminator `\x1b\\`

### Layout Architecture

- Dashboard: 50/50 split (Continue Watching | Preview)
- Search: 60/40 split (Results | Preview)
- Preview Panel: 60/40 split (Image | Description)
- Margins: 3 cells on all sides
- Size: 130% of calculated size (30% increase)

### State Management

- Single context design: `current_image_data`, `current_anime_id`
- Transition tracking: `previous_image_data`, `in_transition` (for future fade effects)
- Cache management: Clear on image change to prevent stale data
- Last rendered area tracking: Used for clearing iTerm2 images

## Common Issues & Solutions

### Issue: Image Stacking/Layering

**Symptom:** Multiple images visible when navigating quickly  
**Cause:** iTerm2 images persist until overwritten  
**Solution:** Clear area with spaces before each render

### Issue: Duplicate/Misplaced Images in Kitty

**Symptom:** The same preview appears twice, with one copy offset outside the preview pane.  
**Cause:** Kitty action `a=T` transmits and displays immediately, then ani-tui sent `a=p` to place the same image again.  
**Solution:** Transmit with `a=t` and use only the explicit placement command to draw.

### Issue: Windows Terminal Image Corruption

**Symptom:** Images flicker, fail to clear, or render inconsistently in Windows Terminal.  
**Cause:** Windows Terminal graphics support varies by version and environment.  
**Solution:** Default to Halfblocks on Windows Terminal. Recommend Kitty or WezTerm for normal image previews, or use `ANI_TUI_IMAGE_PROTOCOL` to force a renderer after manual verification.

### Issue: Portrait Posters Look Too Narrow

**Symptom:** Anime posters fill the preview height but look compressed.  
**Cause:** Image pixel aspect ratio was applied directly to terminal cells, but cells are taller than they are wide.  
**Solution:** Convert pixel aspect ratio to terminal-cell aspect ratio before calculating preview dimensions.

### Issue: Dashboard Image in Search

**Symptom:** Previous dashboard image visible when entering empty search  
**Cause:** iTerm2 clear_graphics() was no-op  
**Solution:** Implement area-specific clearing for iTerm2 protocol using `last_rendered_area`

### Issue: Flickering

**Symptom:** Image flashes when updating  
**Status:** FIXED in v3.7.0  
**Solution:** Implemented 30fps frame rate limiting with `MIN_RENDER_INTERVAL_MS = 33ms`

**Root Cause:** TUI redraws at ~100ms tick rate but image render had no throttle. iTerm2 protocol was clearing+redrawing area every frame even when image unchanged.

**Implementation:**
- Added `last_render_time: Instant` field to ImageRenderer
- Check `elapsed < MIN_RENDER_INTERVAL_MS` before rendering
- Skip render if too soon (unless first render)
- Secondary throttle in App's `render_image_with_ratatui()` method
- Reset timer in `clear_cache()` to allow immediate render after cache clear

**Result:** Warp terminal no longer flashes; image updates feel smooth even during rapid navigation

## Implementation Details

### Image Sizing with Margins

```rust
const MARGIN: u16 = 3;  // 3 cells on all sides
const SIZE_INCREASE: f32 = 1.3;  // 30% bigger

// Calculate available space after margins
let available_width = area.width.saturating_sub(MARGIN * 2);
let available_height = area.height.saturating_sub(MARGIN * 2);

// Calculate base size then increase by 30%
let (base_cols, base_rows) = calculate_display_size(...);
let display_cols = ((base_cols as f32) * SIZE_INCREASE) as u32;
let display_rows = ((base_rows as f32) * SIZE_INCREASE) as u32;

// Position with margin from top/left, centered in remaining space
let start_x = area.x + MARGIN + (available_width - display_cols as u16) / 2;
let start_y = area.y + MARGIN + (available_height - display_rows as u16) / 2;
```

### Screen Transition Clearing

When switching from Dashboard to Search (Shift+S):

1. Call `clear_terminal_graphics()` FIRST (before clearing cache)
2. This clears the last rendered area for iTerm2 protocol
3. Then clear state variables and cache
4. Switch to Search screen

```rust
// Clear terminal graphics FIRST (before clearing cache)
let _ = self.image_renderer.clear_terminal_graphics();

// Then clear state
self.current_image_data = None;
self.image_renderer.clear_cache();
```

## API Reference

### ImageRenderer

- `new()` - Auto-detect protocol
- `render(data, area)` - Render image to area
- `clear_cache()` - Clear internal cache
- `clear_terminal_graphics()` - Clear all graphics (Kitty/iTerm2 with area)
- `clear_area(area)` - Clear specific area (all protocols)
- `requires_terminal_clear()` - Returns true for Kitty and iTerm2

### PreviewPanel

- `render(frame, area, anime, app)` - Render anime preview
- Uses 60/40 layout (image/description)
- Handles empty state with placeholder

## Future Improvements

### Flickering Reduction

Options researched:

1. **Frame Rate Limiting** - Throttle renders to 30fps
2. **Double Buffering** - Render to off-screen buffer first
3. **Dirty Region Tracking** - Only re-render changed areas
4. **Async Image Loading** - Decouple loading from rendering

### Fade Transitions

- Infrastructure exists: `previous_image_data`, `in_transition`, `transition_progress`
- Challenge: Terminal graphics don't support alpha blending
- Approach: Would need custom dithering or protocol-specific solutions

## Testing Notes

- Test on Warp (iTerm2 protocol)
- Test on Kitty (Kitty protocol)
- Test on Terminal.app (no images)
- Verify clear behavior on rapid navigation
- Check memory usage with large image cache
- Verify margins are consistent on all sides
- Verify 30% size increase is applied correctly

## Changelog

### Pending Changes

When modifying `src/ui/image_renderer.rs`, please document changes here before committing.
Use the helper script: `./scripts/update-image-docs.sh`

### v3.8.3 (2026-05-16)

- Made image protocol selection safer and configurable
  - What: Added `ANI_TUI_IMAGE_PROTOCOL` and defaulted Windows Terminal/unknown terminals to Halfblocks
  - Why: Windows Terminal and some Linux terminals can mishandle terminal graphics protocols
  - Impact: Stable previews by default, with Kitty/WezTerm recommended for full image rendering
- Fixed duplicate Kitty image placement
  - What: Changed Kitty image transmission from `a=T` to `a=t`
  - Why: `a=T` displayed immediately before explicit placement
  - Impact: Kitty shows one preview in the intended pane
- Fixed portrait poster sizing
  - What: Adjusted display sizing for terminal-cell aspect ratio
  - Why: Character cells are taller than they are wide
  - Impact: Posters render with more natural proportions

**Template:**
```
### vX.Y.Z (YYYY-MM-DD)

- [Brief description of change]
  - What: [technical details]
  - Why: [reason/motivation]
  - Impact: [user-visible effects]
```

### v3.7.0

- Implemented 30fps frame rate limiting to reduce flickering on all terminals
- Added `MIN_RENDER_INTERVAL_MS` constant (33ms) to throttle image renders
- Added `last_render_time` tracking in ImageRenderer for rate limiting
- Added secondary throttle in App's `render_image_with_ratatui()` method
- Optimized render path to skip unnecessary redraws when image hasn't changed
- Fixed Warp terminal flashy rendering issues through frame rate capping
- Updated `clear_cache()` to reset `last_render_time` for immediate next render
- Improved iTerm2 protocol stability on macOS terminals (Warp, iTerm2)

### v3.5.0

- Added 3-cell margins on all sides for consistent spacing
- Increased image size by 30% (1.3x factor)
- Fixed iTerm2 image clearing on screen transitions
- Updated `clear_terminal_graphics()` to use `last_rendered_area`
- Added `clear_area()` method for explicit area clearing
- Removed unused `clear_image_area()` method
- Fixed compiler warnings

### v3.4.0

- Single context refactor, unified image state
- Removed separate `dashboard_image_data` and `search_image_data`
- Added transition tracking fields

### v3.3.0

- Added iTerm2 protocol support for Warp
- Implemented area clearing with spaces to prevent stacking

### v3.2.0

- Initial Kitty protocol implementation
- Basic image rendering support

## References

- [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [iTerm2 Inline Images](https://iterm2.com/documentation-images.html)
- [Sixel Graphics](https://en.wikipedia.org/wiki/Sixel)
- [Ratatui](https://github.com/ratatui/ratatui) - TUI framework
