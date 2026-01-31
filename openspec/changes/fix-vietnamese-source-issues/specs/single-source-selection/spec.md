## ADDED Requirements

### Requirement: Single source selection interface
The system SHALL only allow users to select one anime source at a time.

#### Scenario: User selects English source
- **WHEN** user is on the source selection screen
- **AND** user presses Space or Enter on the English option
- **THEN** only English source becomes selected
- **AND** Vietnamese source becomes unselected

#### Scenario: User selects Vietnamese source
- **WHEN** user is on the source selection screen
- **AND** user presses Space or Enter on the Vietnamese option
- **THEN** only Vietnamese source becomes selected
- **AND** English source becomes unselected

### Requirement: Source selection persistence
The system SHALL remember the user's source selection across sessions.

#### Scenario: App restart preserves selection
- **WHEN** user selects Vietnamese source
- **AND** user restarts the application
- **THEN** the source selection screen shows Vietnamese as selected

### Requirement: Minimum one source requirement
The system SHALL require at least one source to be enabled at all times.

#### Scenario: Cannot disable last source
- **WHEN** only one source is currently selected
- **AND** user attempts to disable that source
- **THEN** the system prevents the action
- **AND** displays a warning message