# Image Rendering in ani-tui

ani-tui v3.7.4 supports image previews in the terminal using multiple graphics protocols. This document explains how image rendering works and which terminals are supported.

> **Note**: Image rendering is automatically detected and configured. No manual setup required.

## Supported Protocols

### 1. Kitty Graphics Protocol
**Best quality, stateful protocol**

- **Supported terminals**: Kitty, Ghostty
- **Features**: Native terminal graphics, high quality, fast rendering
- **Implementation**: Uses escape sequences with base64-encoded PNG data
- **Image ID**: Fixed at 1 to prevent accumulation

### 2. iTerm2 Inline Images Protocol
**Widely supported, stateless**

- **Supported terminals**: iTerm2, Warp, WezTerm, VSCode, Windows Terminal 1.22+
- **Features**: Broad compatibility, no external dependencies
- **Implementation**: OSC 1337 escape sequences with base64 data
- **Cell size**: 8x16 pixels assumed for calculations

### 3. Sixel Graphics
**Fallback via chafa**

- **Supported terminals**: Any terminal with chafa installed
- **Features**: Works almost everywhere with chafa
- **Implementation**: Converts images to Sixel format via external chafa process
- **Requirements**: `chafa` must be installed

### 4. None
**No image support**

- **Fallback**: Terminal.app and unsupported terminals
- **Behavior**: Shows text placeholder with helpful message

## Protocol Detection

The renderer auto-detects the best protocol based on environment variables:

```rust
Detection priority:
1. Terminal.app (Apple_Terminal) → None
2. iTerm2 (iTerm.app) → iTerm2
3. Warp (WarpTerminal/WARP_SESSION_ID) → iTerm2
4. WezTerm (WezTerm) → iTerm2
5. Kitty (xterm-kitty/KITTY_WINDOW_ID) → Kitty
6. Ghostty (Ghostty/ghostty) → Kitty
7. Windows Terminal (WT_SESSION) → iTerm2 (v1.22+) or Sixel
8. Fallback with chafa → Sixel
9. No chafa → None
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
// Maintains aspect ratio within available cell space
aspect_ratio = img_width / img_height
cols = available_cols
rows = cols / aspect_ratio
// Clamp to available space
```

### Rendering Intervals
Minimum 33ms between renders to prevent flickering and excessive CPU usage.

### Cache Management
- **Sixel**: Cached by area size for reuse
- **Kitty/iTerm2**: Hash-based deduplication
- **Clear**: Called on screen transitions

## Terminal-Specific Notes

### macOS
- **iTerm2**: Best native experience
- **Warp**: Uses iTerm2 protocol (avoids Kitty corruption issues)
- **Terminal.app**: No image support (shows help message)

### Windows
- **Windows Terminal 1.22+**: Native iTerm2 support
- **Older versions**: Requires chafa for Sixel
- **WezTerm**: Full iTerm2 support

### Linux
- **Kitty**: Best Kitty protocol support
- **WezTerm**: Good iTerm2 support
- **GNOME Terminal/Konsole**: Requires chafa

## Troubleshooting

### Images not displaying
1. Check terminal support (see Supported Terminals)
2. Install chafa: `brew install chafa` / `scoop install chafa`
3. Verify image format is supported
4. Try resizing terminal (minimum 10x5 cells needed)

### Flickering or corruption
- Kitty in Warp: Known issue, use iTerm2 protocol instead
- Too many images: Cache clears automatically
- Resize issues: Images clear on terminal resize

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
