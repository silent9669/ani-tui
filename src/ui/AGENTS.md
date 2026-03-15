# UI Module - AGENTS.md

## Overview
TUI components using ratatui. Main app loop, screens, image rendering, and player controls.

## Structure
```
ui/
├── app.rs              # Main app loop (2653 lines) - Screen management, event handling
├── image_renderer.rs   # Terminal image rendering (873 lines) - Kitty/iTerm2/Sixel protocols
├── modern_components.rs # UI widgets (511 lines) - PreviewPanel, SearchOverlay, SplashScreen
├── player_controller.rs # Player UI (291 lines) - Controls overlay, state management
├── image_display.rs    # Image encoding helpers (127 lines)
├── ascii_art.rs        # Splash screen art (81 lines)
├── components/         # Reusable widgets (LoadingSpinner, Toast, episode_grid)
└── mod.rs              # Module exports
```

## Key Components

### App (app.rs)
- **Screens**: Splash → SourceSelect → Home → Search → EpisodeSelect → Player
- **State management**: Current screen, search query, selected anime, episode list
- **Event loop**: Crossterm events → Screen-specific handlers
- **Image handling**: ImageRenderer for cover art previews

### Image Rendering (image_renderer.rs)
**Protocols supported**:
- Kitty: Best quality (Kitty, Ghostty terminals)
- iTerm2: Widely supported (iTerm2, Warp, WezTerm, VSCode)
- Sixel: Fallback via chafa tool
- None: Text placeholder for unsupported terminals

**Auto-detection**: Uses TERM_PROGRAM, TERM, environment variables

### Player Controller
- Control overlay with progress bar
- Next/previous episode buttons
- Episode list integration
- End screen with replay/next options

## Conventions

### Screen Pattern
```rust
pub enum Screen {
    Home,
    Search,
    // ...
}

// In app.rs
match self.current_screen {
    Screen::Home => self.render_home(f),
    // ...
}
```

### Image Rendering Flow
1. Load image bytes from URL
2. Detect terminal protocol on startup
3. Calculate display size (maintain aspect ratio)
4. Render via protocol-specific escape sequences
5. Clear on screen transition

### UI Components
- Use `ratatui` widgets (Block, Paragraph, List)
- Style with `Style::default().fg(Color::Cyan)`
- Layout with `Layout::default().constraints([...])`

## Anti-Patterns
- **DON'T** render images too frequently (min 33ms interval enforced)
- **DON'T** use Kitty protocol in Warp (use iTerm2 instead - avoids corruption)
- **DON'T** forget to clear images on screen transitions
- **DON'T** block the event loop with long operations (use async)

## Where to Look
| Task | File |
|------|------|
| Add new screen | app.rs - add to Screen enum, render method, event handler |
| Change image rendering | image_renderer.rs - Protocol enum, render methods |
| New UI component | modern_components.rs or components/ |
| Player controls | player_controller.rs |
| Splash screen | ascii_art.rs, modern_components.rs SplashScreen |

## Dependencies
- `ratatui` - TUI framework
- `crossterm` - Terminal events and control
- `image` - Image dimension extraction
- `base64` - Protocol encoding
