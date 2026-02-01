## ADDED Requirements

### Requirement: Search preview panel uses portrait orientation
The Search preview panel SHALL use a portrait-oriented layout with 70% of vertical space allocated to the image and 30% allocated to the description text.

#### Scenario: Search results display
- **WHEN** the user navigates to the search screen and selects an anime
- **THEN** the preview panel SHALL display the cover image using 70% of the available vertical height
- **AND** the remaining 30% SHALL display anime information (title, episodes, rating, genres, description)

#### Scenario: Anime cover aspect ratio
- **WHEN** rendering anime cover art in the preview panel
- **THEN** the image SHALL be rendered with sufficient height for chafa to display recognizable cover art
- **AND** the aspect ratio SHALL favor portrait orientation over landscape

### Requirement: Dashboard preview panel uses portrait orientation
The Dashboard continue watching preview panel SHALL use a portrait-oriented layout with 60% of vertical space allocated to the image and 40% allocated to the description text.

#### Scenario: Continue watching selection
- **WHEN** the user navigates the continue watching list on the dashboard
- **THEN** the preview panel SHALL display the anime cover image using 60% of the available vertical height
- **AND** the remaining 40% SHALL display anime information (title, episode, provider)

#### Scenario: Cover image loading
- **WHEN** the user selects a different anime from the continue watching list
- **THEN** the system SHALL load and display the cover image for the selected anime
- **AND** the image SHALL fill the 60% allocated height area

### Requirement: Description text fills available space
The description and metadata text SHALL fill the allocated description area without arbitrary line limits.

#### Scenario: Description display
- **WHEN** displaying anime information in the preview panel
- **THEN** the system SHALL display as much text as fits within the allocated description area
- **AND** text SHALL wrap naturally within the available width
- **AND** no maximum line count SHALL be enforced

### Requirement: Cross-platform visual consistency
The portrait image preview layout SHALL render identically on all supported platforms (macOS, Windows, Linux).

#### Scenario: macOS rendering
- **WHEN** the application runs on macOS (Intel or Apple Silicon)
- **THEN** the preview panel SHALL use the same 70%/30% or 60%/40% ratios as other platforms
- **AND** chafa SHALL render images with equivalent quality

#### Scenario: Windows rendering
- **WHEN** the application runs on Windows
- **THEN** the preview panel SHALL use the same layout ratios as macOS
- **AND** image rendering SHALL be consistent with macOS behavior

#### Scenario: Linux rendering
- **WHEN** the application runs on Linux
- **THEN** the preview panel SHALL use the same layout ratios as other platforms
- **AND** the visual appearance SHALL match macOS and Windows versions
