## REMOVED Requirements

### Requirement: Picker rows can carry shortcut actions

## ADDED Requirements

### Requirement: Picker rows carry Ctrl-key actions

The system SHALL allow a picker row to declare Ctrl-key actions, SHALL show each key and its action in the footer bar of the picker, SHALL run the action against the highlighted row without closing the picker, and SHALL keep every letter typing into the filter when the filter is not empty.

#### Scenario: Row shortcut
- **WHEN** the session picker highlights a row
- **THEN** Ctrl+R opens the rename prompt for that row and the picker returns afterwards

#### Scenario: Filter wins
- **WHEN** the user types `r` in the session picker
- **THEN** `r` filters the list rather than renaming

#### Scenario: Ctrl+D exits from a picker
- **WHEN** a picker is open and the user presses Ctrl+D
- **THEN** the application exits and no row action runs

#### Scenario: Delete on Ctrl+X
- **WHEN** the session picker highlights a session and the user presses Ctrl+X
- **THEN** the session's delete action runs and the picker stays open

#### Scenario: Footer
- **WHEN** a picker has row actions
- **THEN** the footer bar lists each key and its action

## MODIFIED Requirements

### Requirement: The modal footer shows shortcut keys

The system SHALL draw a footer bar at the bottom of a picker box when the picker declares row actions, SHALL fill the bar with the panel background, SHALL list each action as its name and its key with the key muted, SHALL place the first action at the left and the others at the right in declaration order, and SHALL draw no key-hint text elsewhere in the picker.

#### Scenario: Sessions footer
- **WHEN** the session picker is open
- **THEN** the footer bar lists `current directory ctrl+a` at the left and `new ctrl+n`, `delete ctrl+x`, and `rename ctrl+r` at the right

#### Scenario: No actions
- **WHEN** a picker declares no row actions
- **THEN** no footer bar is drawn and the box keeps its height
