## ADDED Requirements

### Requirement: Inline image display with Kitty protocol
The system SHALL display anime cover images using the Kitty inline image protocol when the terminal supports it.

#### Scenario: Image display on Kitty-compatible terminal
- **WHEN** the user views an anime in search results or continue watching
- **AND** the terminal is WezTerm, Windows Terminal, iTerm2, Alacritty, or Kitty
- **THEN** the system SHALL render the cover image using Kitty protocol
- **AND** the image SHALL be displayed in portrait orientation

#### Scenario: No image display on unsupported terminal
- **WHEN** the user views an anime on Terminal.app, older PowerShell, or terminal without image support
- **THEN** the system SHALL display a placeholder text "[Image]"
- **AND** no image escape sequences SHALL be sent

### Requirement: Portrait-oriented image layout
The system SHALL use portrait-oriented layout for image preview panels to optimize cover art display.

#### Scenario: Search preview panel layout
- **WHEN** the user performs a search and selects an anime
- **THEN** the preview panel SHALL allocate 70% of vertical space to the image
- **AND** the remaining 30% SHALL display anime information

#### Scenario: Dashboard preview panel layout
- **WHEN** the user is on the dashboard and selects a continue watching item
- **THEN** the preview panel SHALL allocate 60% of vertical space to the image
- **AND** the remaining 40% SHALL display anime information

### Requirement: Automatic terminal detection
The system SHALL automatically detect terminal capabilities and choose appropriate display mode.

#### Scenario: Detect WezTerm
- **WHEN** the system starts
- **AND** `TERM_PROGRAM` environment variable is "WezTerm"
- **THEN** the system SHALL use Kitty protocol for image display

#### Scenario: Detect Windows Terminal
- **WHEN** the system starts
- **AND** `WT_SESSION` environment variable is set
- **THEN** the system SHALL use Kitty protocol for image display

#### Scenario: Detect iTerm2
- **WHEN** the system starts
- **AND** `TERM_PROGRAM` is "Apple_Terminal" with `ITERM_PROFILE` set
- **THEN** the system SHALL use iTerm2 protocol for image display

#### Scenario: Detect unsupported terminal
- **WHEN** the system starts
- **AND** none of the supported terminal indicators are present
- **THEN** the system SHALL use placeholder mode
- **AND** SHALL display "[Image]" instead of real images

### Requirement: Image data handling
The system SHALL handle image data properly for display, including format validation and sizing.

#### Scenario: Valid image format
- **WHEN** image data is received from the image pipeline
- **AND** the format is PNG, JPEG, GIF, BMP, or WEBP
- **THEN** the system SHALL prepare the image for display
- **AND** the image SHALL be scaled to fit the allocated panel space

#### Scenario: Invalid or empty image data
- **WHEN** image data is empty or not a valid image format
- **THEN** the system SHALL display the placeholder "[Image]"
- **AND** no error SHALL be shown to the user

### Requirement: Image display performance
The system SHALL display images efficiently without blocking the UI.

#### Scenario: Image loads while navigating
- **WHEN** the user navigates between anime in search results
- **THEN** the image display SHALL update within 100ms of selection
- **AND** the UI SHALL remain responsive during image loading

#### Scenario: Image cache hit
- **WHEN** the user revisits an anime already loaded
- **THEN** the image SHALL display from cache immediately
- **AND** no network request SHALL be made
