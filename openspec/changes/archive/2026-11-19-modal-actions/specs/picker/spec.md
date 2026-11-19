# Spec Delta

## ADDED Requirements

### Requirement: Picker rows can carry shortcut actions

The system SHALL allow a picker row to declare single-key actions, SHALL show those keys in a footer hint with the row's action names, SHALL run the action against the highlighted row without closing the picker, and SHALL keep typing filtering when the filter is not empty.

#### Scenario: Row shortcut
- **WHEN** the session picker highlights a row and the filter is empty
- **THEN** `r` opens the rename prompt for that row and the picker returns afterwards

#### Scenario: Filter wins
- **WHEN** the filter contains text and the user types `r`
- **THEN** `r` filters the list rather than renaming

#### Scenario: Footer
- **WHEN** a picker has row actions
- **THEN** the footer lists each key and its action next to Enter and Esc
