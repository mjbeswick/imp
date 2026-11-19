# Spec Delta

## ADDED Requirements

### Requirement: File changes show a diff

After a successful `edit` or `write`, the system SHALL draw the changed lines under the tool row, SHALL mark removed lines in the delete colour and added lines in the add colour, SHALL show at most 12 lines by default with a count of the rest, SHALL show the complete diff when the row is expanded with Ctrl+O, and SHALL keep the diff out of the tool result sent to the model.

#### Scenario: Edit diff
- **WHEN** an edit replaces two lines with three
- **THEN** the row shows the two removed lines in red and the three added lines in green

#### Scenario: New file
- **WHEN** `write` creates a file with 20 lines
- **THEN** the preview shows the first 12 as additions and says 8 more follow

#### Scenario: Overwrite
- **WHEN** `write` replaces an existing file
- **THEN** the diff shows the written content as added lines

#### Scenario: Expand
- **WHEN** the user presses Ctrl+O after an edit
- **THEN** the full diff appears, not only the 12-line preview

#### Scenario: No colour
- **WHEN** colour is off
- **THEN** removed lines begin with `-` and added lines with `+`, and the layout is unchanged

#### Scenario: Model result unchanged
- **WHEN** an edit succeeds
- **THEN** the tool result the model receives is still the short `edited <path>` sentence

#### Scenario: Failure
- **WHEN** an edit is refused because a match failed
- **THEN** no diff is drawn and the existing error preview is shown
