## Why

The current image preview layout uses landscape-oriented ratios (40-50% image height), which reduces the quality of chafa-rendered images. By switching to portrait-oriented layouts (60-70% image height), we can significantly improve image clarity while maintaining a functional text description area.

## What Changes

- **Search Preview Panel**: Change layout from 40%/60% to **70% image / 30% description**
- **Dashboard Continue Watching Preview**: Change layout from 50%/50% to **60% image / 40% description**
- **Description Display**: No line limits - display as much description text as fits in the allocated space
- **Windows Installer**: Verify and improve chafa installation reliability with scoop fallback
- **Cross-Platform Consistency**: Ensure both macOS and Windows versions render identically

## Capabilities

### New Capabilities
- `portrait-image-preview`: Define portrait-oriented layout specifications for image preview panels with optimal aspect ratios for cover art display

### Modified Capabilities
- (none - this is purely implementation/layout change)

## Impact

**Affected Files:**
- `src/ui/modern_components.rs` - PreviewPanel layout adjustment
- `src/ui/app.rs` - Continue watching preview layout adjustment
- `packaging/windows/install-complete.ps1` - Dependency installation improvements

**Dependencies:**
- No new dependencies
- chafa (existing) - continues to be used for image rendering
- mpv (existing) - video player dependency unchanged

**Platforms:**
- macOS (Intel/Apple Silicon): No changes needed, already passes brew test on 14, 15, 26
- Windows: Improved installer reliability for chafa installation
- Linux: No changes needed

**Backwards Compatibility:**
- No breaking changes
- UI layout change only - all existing functionality preserved
