## Context

The current image preview implementation uses landscape-oriented layout ratios (40-50% image height, 60-50% description height). This creates problems because:

1. **Chafa rendering quality**: The chafa image renderer needs sufficient height to display recognizable ASCII art. Landscape orientation with limited height produces poor quality, unreadable images.

2. **Cover art aspect ratio**: Anime cover art is typically portrait-oriented (tall rectangles). The current layout doesn't respect this natural aspect ratio, causing the image to be compressed.

3. **User experience**: Users can't clearly identify anime from the preview images, reducing the utility of the preview panel.

**Current layout:**
- Search Preview: 40% image / 60% description
- Dashboard Preview: 50% image / 50% description

**Stakeholders:**
- macOS users (Intel/Apple Silicon)
- Windows users
- Linux users
- Homebrew maintainers (must pass brew test on macOS 14, 15, 26)

## Goals / Non-Goals

**Goals:**
- Improve image preview quality by using portrait-oriented layout ratios
- Ensure consistent behavior across all platforms (macOS, Windows, Linux)
- Maintain functional description display area
- No breaking changes to existing functionality
- Pass Homebrew test on macOS 14, 15, 26

**Non-Goals:**
- No changes to chafa rendering algorithm or configuration
- No changes to database schema or data models
- No new external dependencies
- No changes to video playback functionality
- No authentication or authorization changes

## Decisions

### Decision 1: Portrait Layout Ratios

**Chosen:** Search Preview = 70% image / 30% description, Dashboard Preview = 60% image / 40% description

**Rationale:**
- 70% image height provides sufficient vertical pixels for chafa to render recognizable cover art
- 30-40% description area still allows displaying title, episode, provider, and key metadata
- Dashboard uses slightly less aggressive ratio (60/40) because it has less metadata to display
- The ratios are different because:
  - Search preview includes: title, episodes, rating, genres, full description
  - Dashboard preview includes: title, episode number, provider (simpler)

**Alternatives considered:**
- 80%/20%: Would squeeze description too much, user might miss key info
- 50%/50% (current): Image quality suffers, no improvement over status quo
- Uniform 65%/35% for both: Doesn't account for different metadata volumes

### Decision 2: Description Display Behavior

**Chosen:** No artificial line limits, fill available space

**Rationale:**
- Using Constraint::Percentage with no max line count naturally fills available space
- Ratatui's Paragraph widget with Wrap::default() handles text wrapping
- Users can scroll mentally by viewing more content in the available area
- Simpler implementation with no need for scrollable text areas

**Alternatives considered:**
- Fixed line count (e.g., 5 lines): Would waste space if description is short
- Scrollable text area: Over-engineering for the current use case
- Truncation with "...": Loses information, poor UX

### Decision 3: Cross-Platform Consistency

**Chosen:** Same layout ratios on all platforms, same chafa rendering approach

**Rationale:**
- ratatui provides consistent layout behavior across platforms
- chafa is available on all platforms (brew, scoop, apt)
- No platform-specific code needed for layout
- The constraint-based layout system handles different terminal emulators

**Implementation approach:**
- Use ratatui's Constraint::Percentage for all layout divisions
- No conditional logic based on platform
- Test on each platform to verify visual consistency

### Decision 4: Windows Installer Chafa Installation

**Chosen:** Use winget with scoop fallback for chafa

**Rationale:**
- winget is the standard Windows package manager
- `hpjansson.chafa` is the official chafa package on winget
- scoop provides alternative for users without winget access
- Same approach as mpv installation (consistency)

**Implementation:**
- Try winget first: `winget install hpjansson.chafa`
- Fallback to scoop: `scoop install chafa`
- If both fail, show warning but continue (chafa is optional for core functionality)

## Risks / Trade-offs

**[Risk] Image quality still poor on small terminals**
→ **Mitigation:** Users with very small terminals (< 40 rows) may still have poor image quality. This is an inherent limitation of ASCII art rendering. The improvement is most noticeable on terminals with 50+ rows.

**[Risk] Description text may be too compressed on narrow terminals**
→ **Mitigation:** The description area uses remaining space after image. On very narrow terminals (80 chars), text wrapping may still be readable. Users can resize terminals for better experience.

**[Risk] Windows chafa installation may fail**
→ **Mitigation:** chafa is optional - image previews won't work, but all other functionality remains. Installer shows clear error message with manual installation instructions.

**[Risk] Different chafa versions may render differently**
→ **Mitigation:** The chafa command-line interface is stable across versions. Core options (--size, --format, --symbols) work consistently. Using specific version pins in installers would add maintenance burden.

## Migration Plan

**Deployment:**
1. Merge changes to `src/ui/modern_components.rs` and `src/ui/app.rs`
2. Test on macOS (Intel and Apple Silicon)
3. Test on Windows
4. Test on Linux
5. Create GitHub release with new version tag
6. Homebrew formula auto-updates on tag push

**Rollback:**
1. Revert to previous commit
2. Push new version tag
3. Homebrew formula auto-updates

**No database migration needed:** Layout-only change, no persistent data affected.

## Open Questions

1. Should we add a configuration option for users to customize the layout ratios?
   - Current decision: No, keep it simple. Default ratios work well for most users.

2. Should we support disabling image previews entirely for users who prefer text-only?
   - Current decision: Not in scope. Users can already not install chafa to achieve similar effect.
