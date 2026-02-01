## 1. Research and Setup

- [x] 1.1 Review ratatui-image v1.0 documentation and examples
- [x] 1.2 Verify current Cargo.toml dependencies
- [x] 1.3 Check existing image_display.rs terminal detection code

## 2. Update Search Preview Panel

- [x] 2.1 Remove render_image_with_ascii() function from modern_components.rs
- [x] 2.2 Import ratatui-image crate
- [x] 2.3 Create new image rendering function using ratatui-image with Kitty protocol
- [x] 2.4 Update PreviewPanel::render() to use new image rendering
- [x] 2.5 Ensure portrait layout (70% image / 30% text) is maintained

## 3. Update Dashboard Preview Panel

- [x] 3.1 Remove render_image_with_ascii() function from app.rs
- [x] 3.2 Add ratatui-image import to app.rs
- [x] 3.3 Create new image rendering function for dashboard preview
- [x] 3.4 Update draw_continue_watching_preview() to use new rendering
- [x] 3.5 Ensure portrait layout (60% image / 40% text) is maintained

## 4. Terminal Detection and Fallback

- [x] 4.1 Keep existing image_display.rs terminal detection
- [x] 4.2 Add fallback to show "[Image]" placeholder for unsupported terminals
- [ ] 4.3 Test terminal detection on macOS (WezTerm, iTerm2, Terminal.app)
- [ ] 4.4 Test terminal detection on Windows (Windows Terminal, WezTerm)

## 5. Windows Installer Verification

- [ ] 5.1 Review install-complete.ps1 for all dependencies
- [ ] 5.2 Verify Visual C++ Redistributable installation
- [ ] 5.3 Verify mpv installation (winget + portable fallback)
- [ ] 5.4 Verify chafa installation (winget + scoop fallback)
- [ ] 5.5 Test installer on Windows 11 if possible

## 6. Build and Test

- [x] 6.1 Run cargo build --release
- [x] 6.2 Fix any compilation errors
- [ ] 6.3 Test on macOS with WezTerm (should show real images)
- [ ] 6.4 Test on macOS with Terminal.app (should show placeholder)
- [ ] 6.5 Verify --version command still works (Homebrew compatibility)

## 7. Final Polish

- [ ] 7.1 Remove unused image_ascii dependency if not needed elsewhere
- [ ] 7.2 Clean up any duplicate code
- [ ] 7.3 Add logging for debugging image display issues
- [ ] 7.4 Test navigation (Shift+S, ESC) still works correctly
- [ ] 7.5 Final cargo check and build verification
