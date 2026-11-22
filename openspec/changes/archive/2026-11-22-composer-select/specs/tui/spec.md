## ADDED Requirements

### Requirement: The draft can be selected

The system SHALL extend a selection in the draft with Shift+←/→ by a character, Shift+↑/↓ by a row, and Shift+Home/End to the draft's start or end, SHALL clear it on any other key, SHALL draw it reversed, SHALL remove it on Backspace or Delete, and SHALL replace it with typed text, a newline, or a Ctrl+V paste.

#### Scenario: Replace a word
- **WHEN** the draft is `make it red`, the caret is at the end, and the user presses Shift+← three times and types `blue`
- **THEN** the draft is `make it blue`

#### Scenario: Delete a selection
- **WHEN** the user selects two characters and presses Backspace
- **THEN** both are gone and the caret sits where they began

#### Scenario: Move clears
- **WHEN** a selection exists and the user presses → without Shift
- **THEN** the selection is gone and the text is unchanged

#### Scenario: Across rows
- **WHEN** the draft has two rows and the user presses Shift+↑ from the second
- **THEN** the selection covers the end of the first row and the start of the second, and history is not recalled

### Requirement: Delete removes forward

The system SHALL remove the character after the caret on Delete or Shift+Delete when nothing is selected.

#### Scenario: Forward delete
- **WHEN** the draft is `cat` with the caret after `c` and the user presses Delete
- **THEN** the draft is `ct` and the caret stays after `c`

### Requirement: Home and End work under terminal multiplexers

The system SHALL read Home and End whether the terminal sends `CSI H` and `CSI F` or `CSI 1~`, `CSI 7~`, `CSI 4~`, and `CSI 8~`, with or without Shift.

#### Scenario: Under tmux
- **WHEN** imp runs inside tmux and the user presses Home
- **THEN** the caret moves to the start of the draft
