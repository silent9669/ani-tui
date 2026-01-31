## ADDED Requirements

### Requirement: Stream URL fetching for Vietnamese sources
The system SHALL successfully fetch stream URLs for Vietnamese anime episodes.

#### Scenario: Player opens for Vietnamese episode
- **WHEN** user selects an episode from Vietnamese source
- **AND** presses Enter to play
- **THEN** the system fetches stream URL from KKPhim
- **AND** mpv player opens
- **AND** video starts playing

#### Scenario: Player does not hang
- **WHEN** user selects a Vietnamese episode
- **AND** stream URL is being fetched
- **THEN** a loading message is displayed
- **AND** if fetching takes more than 10 seconds, shows timeout error
- **AND** returns to episode selection

### Requirement: Error handling for stream failures
The system SHALL handle stream URL fetch failures gracefully.

#### Scenario: Stream URL not found
- **WHEN** stream URL fetch fails for an episode
- **THEN** an error message is displayed: "Failed to load video"
- **AND** the reason is logged
- **AND** user returns to episode selection screen

#### Scenario: Invalid stream URL
- **WHEN** fetched stream URL is empty or invalid
- **THEN** error message is displayed
- **AND** user can try another episode

### Requirement: Consistent behavior with English source
The system SHALL provide the same streaming experience for Vietnamese as English sources.

#### Scenario: Same episode selection flow
- **WHEN** user plays Vietnamese anime
- **THEN** the episode selection screen works identically to English
- **AND** navigation keys (↑/↓/Enter/Esc) work the same
- **AND** player controls work the same

#### Scenario: Watch history saved
- **WHEN** user watches a Vietnamese episode
- **AND** exits the player
- **THEN** the watch history is saved
- **AND** appears in "Continue Watching" on home screen