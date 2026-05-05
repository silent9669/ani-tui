# Image Rendering System - Development History

## Overview

Cross-platform image preview for ani-tui using multiple terminal graphics protocols.

## Protocols Supported

- **Kitty Graphics Protocol** - Best quality, requires explicit clearing
- **iTerm2 Inline Images** - Widely supported (Warp, iTerm2, WezTerm), stateless
- **Sixel (via chafa)** - Fallback for older terminals
- **None** - Terminal.app has no support

## Key Design Decisions

### Protocol Detection Priority

1. Terminal.app → None (no support)
2. iTerm2 → iTerm2 protocol
3. Warp → iTerm2 protocol (avoids Kitty corruption issues)
4. WezTerm → iTerm2 protocol
5. Kitty → Kitty protocol
6. Windows Terminal → Sixel
7. Default → Sixel (chafa fallback)

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

### Issue: Image Escapes Spill Outside the TUI on Linux Mint

**Symptom:** Preview images draw over other panes, stack on top of each other, or appear outside the terminal UI when searching.
**Cause:** Some Linux terminals do not safely support sixel graphics, even when `chafa` is installed and can generate sixel output.
**Solution:** Unknown terminals now use the native Halfblocks renderer by default. This keeps image previews inside Ratatui's normal buffer and prevents protocol graphics from escaping the layout.

Users with a known-compatible terminal can opt into a graphics protocol:

```bash
ANI_TUI_IMAGE_PROTOCOL=sixel ani-tui
ANI_TUI_IMAGE_PROTOCOL=kitty ani-tui
ANI_TUI_IMAGE_PROTOCOL=iterm2 ani-tui
ANI_TUI_IMAGE_PROTOCOL=halfblocks ani-tui
```

### Issue: Image Stacking/Layering

**Symptom:** Multiple images visible when navigating quickly  
**Cause:** iTerm2 images persist until overwritten  
**Solution:** Clear area with spaces before each render

### Issue: Duplicate/Misplaced Images in Kitty

**Symptom:** The same preview appears twice, with one image offset outside the preview pane.
**Cause:** Kitty action `a=T` transmits and displays immediately, and the renderer then sends `a=p` to place the same image again.
**Solution:** Kitty now transmits image data with `a=t` and uses only the explicit `a=p` placement for display.

### Issue: Portrait Posters Look Too Narrow

**Symptom:** Anime posters fill the preview height but look compressed in width.
**Cause:** Graphics protocol placements are sized in terminal cells, but terminal cells are taller than they are wide. Preserving image pixel aspect ratio directly in cell counts makes portrait images too skinny.
**Solution:** Display sizing now converts image pixel aspect ratio into terminal-cell aspect ratio before calculating columns and rows.

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

### v3.8.3 (2026-05-05)

- Made Linux terminal image rendering safer by default
  - What: Added `ANI_TUI_IMAGE_PROTOCOL` override and changed unknown terminals to use Halfblocks instead of automatically selecting sixel when `chafa` is installed
  - Why: Linux Mint default terminals can display sixel output incorrectly, causing images to spill outside the TUI during search
  - Impact: Mint users get stable in-pane image previews by default; compatible terminal users can still force sixel, Kitty, or iTerm2 rendering
- Fixed duplicate/misplaced Kitty preview images
  - What: Changed Kitty image transmission from `a=T` to `a=t`, keeping `a=p` as the only display action
  - Why: `a=T` already displays the image, so using it before `a=p` created a second placement at the wrong cursor position
  - Impact: Kitty users should see a single preview image inside the intended panel
- Fixed skinny portrait poster sizing
  - What: Adjusted display-size calculation to account for terminal cells being taller than they are wide
  - Why: Pixel aspect ratio was being applied directly to terminal cell counts, compressing poster width
  - Impact: Portrait covers render with a more natural width in Kitty/iTerm2 graphics placements

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
