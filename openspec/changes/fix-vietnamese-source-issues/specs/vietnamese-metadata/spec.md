## ADDED Requirements

### Requirement: AniList metadata for Vietnamese titles
The system SHALL fetch metadata from AniList for Vietnamese anime titles.

#### Scenario: Metadata loads for Vietnamese anime
- **WHEN** user selects a Vietnamese anime in search results
- **THEN** the system searches AniList using the anime title
- **AND** displays the matched metadata in preview panel
- **AND** shows rating, genres, and description

#### Scenario: No "Loading metadata..." permanently
- **WHEN** user navigates to a Vietnamese anime
- **AND** metadata takes time to load
- **THEN** "Loading metadata..." is displayed temporarily
- **AND** actual metadata appears within 3 seconds
- **AND** if no metadata found, shows "Metadata unavailable"

### Requirement: Metadata display
The system SHALL display metadata in the preview panel.

#### Scenario: All metadata fields shown
- **WHEN** metadata is loaded for an anime
- **THEN** the preview panel shows:
  - Title
  - Episode count
  - Rating with stars (e.g., ★★★★☆ 8.4/10)
  - Genres list
  - Description text

#### Scenario: Fallback to base info
- **WHEN** AniList metadata is not available
- **THEN** the preview panel shows base provider info
- **AND** episode count from provider
- **AND** synopsis from provider if available

### Requirement: Metadata caching
The system SHALL cache metadata to avoid repeated API calls.

#### Scenario: Metadata loads from cache
- **WHEN** user views an anime that was viewed before
- **THEN** metadata loads from local cache
- **AND** displays instantly without API call
- **AND** respects 7-day cache TTL