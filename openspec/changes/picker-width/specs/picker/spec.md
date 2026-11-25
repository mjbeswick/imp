## ADDED Requirements

### Requirement: Picker width follows its content

The system SHALL size a picker's modal to fit its widest row, SHALL keep the modal between 72 and 120
columns, SHALL never draw it wider than 90% of the terminal, and SHALL keep the old width on terminals
narrower than 60 columns.

#### Scenario: Long descriptions get room
- **WHEN** the terminal is 200 columns wide and the widest picker row needs 110 columns
- **THEN** the modal is 110 columns wide, and its descriptions are not cut for width

#### Scenario: Short pickers stay at the minimum
- **WHEN** the terminal is 200 columns wide and the widest row needs 60 columns
- **THEN** the modal is 72 columns wide

#### Scenario: Capped at 120
- **WHEN** the terminal is 200 columns wide and the widest row needs 300 columns
- **THEN** the modal is 120 columns wide

#### Scenario: Leaves a margin
- **WHEN** the terminal is 100 columns wide and the widest row needs 110 columns
- **THEN** the modal is 90 columns wide

#### Scenario: Narrow terminal
- **WHEN** the terminal is 50 columns wide
- **THEN** the modal is 46 columns wide, as before
