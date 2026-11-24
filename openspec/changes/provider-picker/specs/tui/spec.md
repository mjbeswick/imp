## ADDED Requirements

### Requirement: Provider rows keep a stable height

The system SHALL draw every row of the provider picker with the same wrapping, whether the row is selected or not, so that moving the selection SHALL NOT change the height of the list.

#### Scenario: Moving down
- **WHEN** the user moves the selection down the provider picker
- **THEN** the rows keep their heights and the list does not jump

### Requirement: Enter on a provider opens its editor

The system SHALL open the provider editor in the modal when the user presses Enter on any provider row, built-in or custom, SHALL NOT change the active provider on Enter, and SHALL keep `u` for switching.

#### Scenario: Enter on custom
- **WHEN** the user selects the `custom` row in the provider picker and presses Enter
- **THEN** the provider editor opens in the modal and the active provider is unchanged

#### Scenario: Switch with u
- **WHEN** the user presses `u` on a provider row
- **THEN** that provider becomes active
