## ADDED Requirements

### Requirement: A paste arrives whole

The system SHALL turn on bracketed paste with the full-screen interface and off when it ends, SHALL insert a paste into the draft at the caret with its line breaks and without sending, SHALL insert it into an open text prompt or answer row with line breaks as spaces, and SHALL ignore a paste while a confirmation is open.

#### Scenario: Several lines
- **WHEN** the user pastes `one\ntwo` into an empty draft
- **THEN** the draft holds both lines and nothing is sent

#### Scenario: Confirmation open
- **WHEN** a confirmation is open and the user pastes `y`
- **THEN** the confirmation stays open and unanswered

### Requirement: Dragging selects and copies

The system SHALL, when `mouse` is on, highlight the text from where a left press began to the pointer while the button is held, SHALL copy that text on release to imp's buffer and the terminal clipboard and say how much was copied, and SHALL treat a press and release on the same cell as a click.

#### Scenario: Drag
- **WHEN** the user drags across `hello world` in the transcript and releases
- **THEN** the screen showed the span reversed while dragging, and Ctrl+V pastes `hello world`

#### Scenario: Click
- **WHEN** the user presses and releases on one cell of a picker row
- **THEN** the row acts as before and nothing is copied
