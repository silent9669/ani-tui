# Image Rendering in ani-tui

ani-tui supports image previews in the terminal using multiple graphics protocols. This document explains how image rendering works and which terminals are supported.

Current app version: 3.8.3.

> **Note**: Image rendering is automatically detected and configured. Set `ANI_TUI_IMAGE_PROTOCOL=kitty|iterm2|sixel|halfblocks|auto` only when you want to override detection.

## Supported Protocols

### 1. Kitty Graphics Protocol
**Best quality, stateful protocol**

- **Supported terminals**: Kitty, Ghostty
- **Features**: Native terminal graphics, high quality, fast rendering
- **Implementation**: Uses escape sequences with base64-encoded PNG data
- **Image ID**: Fixed at 1 to prevent accumulation

### 2. iTerm2 Inline Images Protocol
**Widely supported, stateless**

- **Supported terminals**: iTerm2, Warp, VSCode
- **Features**: Broad compatibility, no external dependencies
- **Implementation**: OSC 1337 escape sequences with base64 data
- **Sizing**: Uses character cells (not pixels) for consistent display across terminals
- **Size multiplier**: 2.5x for better visibility

### 3. Sixel Graphics
**Fallback via chafa**

- **Supported terminals**: Known sixel terminals such as foot, mlterm, and contour, or any terminal when explicitly forced with `ANI_TUI_IMAGE_PROTOCOL=sixel`
- **Features**: Works almost everywhere with chafa
- **Implementation**: Converts images to Sixel format via external chafa process
- **Requirements**: `chafa` must be installed

### 4. Halfblocks
**Stable in-buffer fallback**

- **Fallback**: Terminal.app, Windows Terminal, and unknown terminals
- **Behavior**: Renders previews inside Ratatui's normal text buffer

## Protocol Detection

The renderer auto-detects the best protocol based on environment variables:

```rust
Detection priority:
1. Terminal.app (Apple_Terminal) → Halfblocks
2. iTerm2 (iTerm.app) → iTerm2
3. Warp (WarpTerminal/WARP_SESSION_ID) → iTerm2
4. WezTerm (WezTerm) → Kitty
5. Kitty (xterm-kitty/KITTY_WINDOW_ID) → Kitty
6. Ghostty (Ghostty/ghostty) → Kitty
7. Rio (Rio/rio) → Kitty
8. Windows Terminal (WT_SESSION) → Halfblocks
9. Known sixel terminals with chafa → Sixel
10. Unknown terminals → Halfblocks
```

## Image Rendering Flow

```
1. Detect protocol on startup
2. Load image data (PNG/JPEG/WebP/GIF/BMP)
3. Validate image format (magic bytes check)
4. Calculate display size (maintain aspect ratio)
5. Render via detected protocol
6. Cache rendered output (Sixel only)
7. Clear on screen transition
```

## Key Implementation Details

### Image Validation
```rust
// Supported formats checked via magic bytes
PNG:  [0x89, 0x50, 0x4E, 0x47...]
JPEG: [0xFF, 0xD8, 0xFF]
WebP: [0x52, 0x49, 0x46, 0x46]
GIF:  [0x47, 0x49, 0x46, 0x38]
BMP:  [0x42, 0x4D]
```

### Aspect Ratio Calculation
```rust
// Maintains visual aspect ratio within terminal cells
pixel_aspect_ratio = img_width / img_height
cell_aspect_ratio = pixel_aspect_ratio / 0.5
cols = available_cols
rows = cols / cell_aspect_ratio
// Clamp to available space
```

### Rendering Intervals
Minimum 33ms between renders to prevent flickering and excessive CPU usage.

### Cache Management
- **Sixel**: Cached by area size for reuse
- **Kitty/iTerm2**: Hash-based deduplication
- **Clear**: Called on screen transitions

### Image Clearing on Screen Transitions
Images are cleared when transitioning between screens to prevent stale images:

| From Screen | To Screen | Clear Action |
|-------------|-----------|--------------|
| Home | SourceSelect | ✅ Clear terminal graphics + cache |
| SourceSelect | Search | ✅ Clear terminal graphics + cache |
| Search | Home | ✅ Clear terminal graphics + cache |
| Search | EpisodeSelect | ✅ Clear terminal graphics + cache |
| Player | EpisodeSelect | ✅ Clear terminal graphics + cache |
| EpisodeSelect | Search/Home | ✅ Preview reload triggered |

The `clear_terminal_graphics()` method writes spaces over the image area for iTerm2 protocol, while Kitty protocol sends delete commands.

## Terminal-Specific Notes

### macOS
- **iTerm2**: Best native experience
- **Warp**: Uses iTerm2 protocol (avoids Kitty corruption issues)
- **Terminal.app**: No image support (shows help message)

### Windows
- **Kitty / WezTerm**: Recommended for normal image previews
- **Windows Terminal**: Uses Halfblocks by default for stable rendering
- **Override**: Use `ANI_TUI_IMAGE_PROTOCOL=iterm2` or `sixel` only if you have verified your terminal build handles it well

### Linux
- **Kitty**: Best Kitty protocol support
- **WezTerm**: Uses Kitty protocol
- **foot/mlterm/contour**: Sixel via chafa
- **GNOME Terminal/Konsole/unknown terminals**: Halfblocks by default

## Troubleshooting

### Images not displaying
1. Check terminal support (see Supported Terminals)
2. Try `ANI_TUI_IMAGE_PROTOCOL=halfblocks ani-tui` for a stable fallback
3. Verify image format is supported
4. Try resizing terminal (minimum 10x5 cells needed)

### Flickering or corruption
- Kitty in Warp: Known issue, use iTerm2 protocol instead
- Too many images: Cache clears automatically
- Resize issues: Images clear on terminal resize

### Images persisting on screen
- Fixed in v3.7.4: Images now clear on all screen transitions
- If images appear stale, check that `clear_terminal_graphics()` is called before screen changes
- iTerm2 requires writing spaces over the image area to clear persistent images

### Performance
- Images cached after first render
- 33ms minimum between renders
- Large images scaled down automatically

## Dependencies

- **chafa**: Required for Sixel fallback
- **image crate**: For dimension extraction
- **base64**: For protocol encoding

## Code Location

Implementation: `src/ui/image_renderer.rs`

Key structures:
- `ImageRenderer`: Main renderer with protocol detection
- `Protocol`: Enum of supported protocols
- `ImageError`: Error types with user-friendly messages
