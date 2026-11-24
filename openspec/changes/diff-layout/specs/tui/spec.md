# TUI

## ADDED Requirements

### Requirement: Wide terminals show diffs side by side

The system SHALL draw a file-change diff side by side when the terminal is at least 120 columns wide, with old text on the left and new text on the right, SHALL draw it inline when the terminal is narrower, and SHALL keep the same colours for removed and added lines in both layouts.

#### Scenario: Wide terminal
- **WHEN** an `edit` changes a line and the terminal is 120 columns wide
- **THEN** the diff shows the old line on the left and the new line on the right

#### Scenario: Narrow terminal
- **WHEN** the same edit runs in a 119-column terminal
- **THEN** the diff is drawn inline, as before

#### Scenario: Long line in side-by-side view
- **WHEN** a changed line is longer than its column in the side-by-side view
- **THEN** the line is cut with an ellipsis, and Ctrl+E shows the full inline diff

#### Scenario: Model output unchanged
- **WHEN** a diff is drawn in either layout
- **THEN** the tool result sent to the model is the same as before
