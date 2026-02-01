## 1. Update Search Preview Layout

- [ ] 1.1 Modify `PreviewPanel` layout in `src/ui/modern_components.rs` to use 70% image / 30% description
- [ ] 1.2 Verify Constraint::Percentage(70) for image area and Constraint::Percentage(30) for info area
- [ ] 1.3 Ensure margin(1) is applied for consistent spacing

## 2. Update Dashboard Preview Layout

- [ ] 2.1 Modify `draw_continue_watching_preview` in `src/ui/app.rs` to use 60% image / 40% description
- [ ] 2.2 Verify Constraint::Percentage(60) for image area and Constraint::Percentage(40) for info area
- [ ] 2.3 Remove any line count limits on description text

## 3. Verify Description Display Behavior

- [ ] 3.1 Confirm no artificial line limits are applied to description text
- [ ] 3.2 Verify Paragraph widget uses Wrap::default() for natural text wrapping
- [ ] 3.3 Test that description fills available space without truncation

## 4. Windows Installer Improvements

- [ ] 4.1 Verify chafa winget package ID (`hpjansson.chafa`) is correct
- [ ] 4.2 Add scoop fallback installation for chafa in `packaging/windows/install-complete.ps1`
- [ ] 4.3 Test installer script syntax with PowerShell

## 5. Cross-Platform Testing

- [ ] 5.1 Build and run on macOS (Intel or Apple Silicon)
- [ ] 5.2 Verify layout ratios display correctly on macOS
- [ ] 5.3 Verify image rendering quality improvement
- [ ] 5.4 Build on Linux (cross-compilation or native)
- [ ] 5.5 Document any platform-specific differences

## 6. Final Verification

- [ ] 6.1 Run `cargo check` to verify no compilation errors
- [ ] 6.2 Run `cargo build --release` for production build
- [ ] 6.3 Verify `--version` command works (for Homebrew test compatibility)
- [ ] 6.4 Review code changes for consistency with existing patterns
