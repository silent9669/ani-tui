## ADDED Requirements

### Requirement: Correct episode count from KKPhim
The system SHALL fetch and display the correct number of episodes from KKPhim API.

#### Scenario: Long series shows correct count
- **WHEN** user searches for "Naruto Shippuden"
- **AND** selects the Vietnamese source result
- **THEN** the episode count shows approximately 500 episodes
- **AND** not 1 episode

#### Scenario: Seasonal anime shows correct count
- **WHEN** user searches for a seasonal anime with 12 episodes
- **AND** selects the Vietnamese source result
- **THEN** the episode count shows 12
- **AND** all 12 episodes are available for selection

### Requirement: Episode list fetching
The system SHALL fetch the complete episode list from KKPhim.

#### Scenario: All episodes available
- **WHEN** user selects a Vietnamese anime with 24 episodes
- **AND** navigates to the episode selection screen
- **THEN** all 24 episodes are displayed in the list
- **AND** episodes are sorted by episode number

#### Scenario: Episode metadata included
- **WHEN** episode list is displayed
- **THEN** each episode shows its episode number
- **AND** each episode shows its title if available

### Requirement: Episode selection
The system SHALL allow users to select any episode from the list.

#### Scenario: Select first episode
- **WHEN** user is on episode selection screen
- **AND** user selects episode 1
- **AND** presses Enter
- **THEN** player opens with episode 1

#### Scenario: Select last episode
- **WHEN** user is on episode selection screen
- **AND** user navigates to and selects the last episode
- **AND** presses Enter
- **THEN** player opens with that episode