## ADDED Requirements

### Requirement: Image rendering with chafa
The system SHALL display anime cover images in the preview panel using chafa.

#### Scenario: Image displays correctly
- **WHEN** user navigates to an anime in search results
- **AND** the anime has a cover image URL
- **THEN** the preview panel displays the rendered image
- **AND** the image is visible (not black squares or empty)

#### Scenario: Fallback when chafa unavailable
- **WHEN** user navigates to an anime
- **AND** chafa is not installed on the system
- **THEN** the preview panel shows a placeholder message
- **AND** the message indicates chafa is required for images

### Requirement: Image caching
The system SHALL cache downloaded images to avoid repeated downloads.

#### Scenario: Image loads from cache
- **WHEN** user navigates to an anime that was viewed before
- **THEN** the image loads from local cache
- **AND** the image displays within 100ms

#### Scenario: Image download on first view
- **WHEN** user navigates to an anime for the first time
- **AND** the image is not in cache
- **THEN** the system downloads the image
- **AND** displays "Loading image..." placeholder
- **AND** replaces with actual image when loaded

### Requirement: Image sizing
The system SHALL render images at appropriate size for the terminal.

#### Scenario: Image fits preview area
- **WHEN** image is rendered in preview panel
- **THEN** the image dimensions fit within the allocated 40% height area
- **AND** the image maintains aspect ratio
- **AND** the image is centered in the area