## Context

The current implementation of image preview uses ASCII art rendered via the `image_ascii` crate, which produces difficult-to-read output. Users cannot clearly identify anime from their cover art in the preview panels. The codebase already has `ratatui-image` v1.0 in Cargo.toml but it's not being used for display. We need to integrate proper inline image support using the Kitty protocol that works across WezTerm, Windows Terminal, iTerm2, and Alacritty.

**Current state:**
- `src/ui/modern_components.rs` has `render_image_with_ascii()` that shows ASCII art or placeholder
- `src/ui/app.rs` has similar `render_image_with_ascii()` for dashboard preview
- `src/ui/image_display.rs` exists with Kitty/iTerm2 encoding functions but isn't integrated into the UI
- Terminal detection logic exists in `image_display.rs`

**Constraints:**
- Must work with ratatui v0.24 (already in use)
- Must not break existing functionality (graceful fallback required)
- Must not introduce new external dependencies beyond what's already in Cargo.toml
- Must maintain performance (UI should remain responsive)

**Stakeholders:**
- macOS users (WezTerm, iTerm2 users get real images, Terminal.app users get placeholder)
- Windows users (Windows Terminal, WezTerm users get real images)
- Linux users (WezTerm, Alacritty users get real images)

## Goals / Non-Goals

**Goals:**
1. Display real anime cover images using Kitty protocol in preview panels
2. Use portrait-oriented layout (70% image / 30% text for search, 60% / 40% for dashboard)
3. Automatically detect terminal capabilities and choose appropriate display mode
4. Provide clean placeholder for unsupported terminals
5. Verify Windows installer sets up all dependencies correctly
6. Ensure Homebrew formula continues to pass bot validation

**Non-Goals:**
- No changes to video playback functionality
- No changes to episode selection or player controls
- No new configuration options for image display (auto-detect only)
- No animated image support (static images only)

## Decisions

### Decision 1: Use ratatui-image crate with Kitty protocol

**Chosen:** Use `ratatui-image` v1.0 with Kitty protocol configuration

**Rationale:**
- `ratatui-image` is already in Cargo.toml (v1.0) - no new dependency needed
- Provides native integration with ratatui widgets
- Supports Kitty protocol which works on WezTerm, Windows Terminal, iTerm2, Alacritty
- Maintained crate with good documentation

**Alternatives considered:**
- Use raw escape sequences (in `image_display.rs`) - Would require bypassing ratatui rendering, potential flicker issues
- Create custom widget from scratch - More work, less tested, no benefit over existing crate
- Use `crossterm` image support - Not as well maintained, less terminal coverage

### Decision 2: Keep image_display.rs for terminal detection

**Chosen:** Keep `image_display.rs` for terminal detection, use ratatui-image for actual rendering

**Rationale:**
- Terminal detection logic is already implemented and working
- Can be used to decide whether to use ratatui-image or fallback
- Separation of concerns: detection vs rendering

**Implementation:**
```rust
use ratatui_image::{Picker, ImageView};

fn render_image(frame: &mut Frame, area: Rect, image_data: &[u8]) {
    let protocol = detect_protocol();
    if matches!(protocol, ImageProtocol::Kitty | ImageProtocol::ITerm2) {
        // Use ratatui-image with Kitty protocol
        let picker = Picker::new()
            .protocol(ratatui_image::Protocol::Kitty);
        let image = image::load_from_memory(image_data).unwrap();
        let view = ImageView::new(area, image, picker);
        frame.render_widget(view, area);
    } else {
        // Show placeholder
        show_placeholder(area);
    }
}
```

### Decision 3: Layout constraints

**Chosen:** Keep existing Constraint::Percentage layout (70% / 30% search, 60% / 40% dashboard)

**Rationale:**
- Already implemented and working
- Portrait ratio is optimal for anime cover art
- No changes needed to layout system

### Decision 4: Graceful fallback strategy

**Chosen:** Show "[Image]" placeholder text for unsupported terminals

**Rationale:**
- Simple, clear user feedback
- No ASCII art generation overhead
- Works on all terminals without special support

**Fallback order:**
1. Try ratatui-image with Kitty protocol
2. If terminal doesn't support images, show placeholder
3. No ASCII fallback (removed to simplify code)

## Risks / Trade-offs

**[Risk] ratatui-image compatibility issues**
→ **Mitigation:** Test thoroughly on both platforms. The crate is mature (v1.0) and widely used.

**[Risk] Image sizing on different terminals**
→ **Mitigation:** Use ratatui-image's automatic sizing. Configure with appropriate aspect ratio.

**[Risk] Flicker when switching between images**
→ **Mitigation:** ratatui-image handles double-buffering. Pre-load images where possible.

**[Risk] Windows installer misses dependencies**
→ **Mitigation:** Current installer checks for VC++ Redistributable, mpv, chafa. Test on clean Windows VM.

**[Risk] Homebrew bot fails**
→ **Mitigation:** Formula already passes `--version` test. No changes to binary interface.

## Migration Plan

**Deployment:**
1. Merge code changes to `src/ui/modern_components.rs` and `src/ui/app.rs`
2. Update Windows installer if needed (verify in testing)
3. Create GitHub release v3.5.0
4. Homebrew formula auto-updates on tag push

**Rollback:**
1. Revert to previous commit
2. Push new version tag
3. Homebrew formula auto-updates

**No database migration needed:** Purely visual change, no persistent data affected.

## Open Questions

1. **Should we pre-load images for faster navigation?** Currently images load on-demand. Pre-loading could improve experience but increases memory usage.

2. **Image caching strategy:** Should we cache rendered images or just raw image data? Raw data caching is already implemented.

3. **Maximum image size:** Should we limit image size for very large covers? Default behavior should work, but may need tuning.

4. **Testing on Windows:** Need to verify Windows Terminal displays images correctly. May need to adjust protocol settings.
