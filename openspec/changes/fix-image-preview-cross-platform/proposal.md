## Why

The current image preview uses ASCII art which is difficult to see and provides poor user experience for anime cover art. By using the `ratatui-image` crate with Kitty protocol, we can display real images on modern terminals (WezTerm, Windows Terminal, iTerm2, Alacritty) while maintaining a clean fallback for unsupported terminals. This is essential for the anime cover art preview feature to be visually useful and help users identify shows quickly.

## What Changes

- Integrate `ratatui-image` crate properly for inline image display in preview panels
- Add Kitty protocol support that works on both macOS (WezTerm, iTerm2) and Windows (Windows Terminal, WezTerm)
- Use portrait-oriented layout (70% image / 30% text) for optimal cover art display in search preview
- Use portrait-oriented layout (60% image / 40% text) for optimal cover art display in dashboard preview
- Show "[Image]" placeholder only for terminals without inline image support (Terminal.app, older PowerShell)
- Verify Windows installer properly sets up all dependencies (VC++ Redistributable, mpv, chafa)
- Ensure Homebrew formula continues to pass bot validation

## Capabilities

### New Capabilities
- `image-preview-display`: Inline image display using Kitty protocol via ratatui-image with automatic terminal detection and portrait-oriented layout

### Modified Capabilities
- (none - this is a new feature implementation, not modifying existing requirements)

## Impact

**Affected Files:**
- `src/ui/modern_components.rs` - Replace `render_image_with_ascii()` with ratatui-image widget in PreviewPanel
- `src/ui/app.rs` - Update image rendering in dashboard continue watching preview panel
- `packaging/windows/install-complete.ps1` - Verify all dependencies are properly installed

**Dependencies:**
- `ratatui-image` (already in Cargo.toml as v1.0)
- `image` (already in Cargo.toml with png/jpeg features)
- `base64` (already added)
- No new external dependencies required

**Platforms:**
- macOS: WezTerm, iTerm2 → Real images | Terminal.app → Placeholder "[Image]"
- Windows: Windows Terminal, WezTerm → Real images | Older PowerShell → Placeholder "[Image]"
- Linux: WezTerm, Alacritty → Real images | Other terminals → Placeholder "[Image]"

**Backwards Compatibility:**
- No breaking changes - only visual improvement
- Graceful fallback for unsupported terminals maintains full functionality
- All existing keyboard shortcuts and navigation remain unchanged

**Windows Installer Verification:**
- Visual C++ Redistributable (via winget)
- mpv player (via winget or portable download)
- chafa for ASCII fallback (via winget/scoop)
- ani-tui binary (downloaded from GitHub releases)
