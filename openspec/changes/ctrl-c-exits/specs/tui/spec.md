# Spec Delta

## MODIFIED Requirements

### Requirement: The interface is keyboard only

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, and Ctrl+J for a newline, Esc to cancel, Ctrl+R to retry, Ctrl+O to expand the last tool output, Ctrl+T to expand thinking, Ctrl+H to hide thinking, Ctrl+L to clear the view, and Ctrl+C to exit when idle or to cancel a running turn and then exit on the next press.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Quit guard
- **WHEN** the session is idle and the user presses Ctrl+C
- **THEN** the app exits on that press

#### Scenario: Cancel then exit
- **WHEN** a turn is running and the user presses Ctrl+C
- **THEN** the turn is cancelled, the status row offers a second press to exit, and pressing Ctrl+C again exits even if the turn has not finished unwinding

#### Scenario: Start a second line
- **WHEN** the user types a line and presses Shift+Enter
- **THEN** the caret moves to a new row, nothing is sent, and the first line stays in the composer

#### Scenario: Terminals without modifier reporting
- **WHEN** the terminal never reports Shift+Enter separately
- **THEN** Alt+Enter and Ctrl+J still start a new line

#### Scenario: Cancel a running turn
- **WHEN** the user presses Esc while a turn is streaming or running a tool
- **THEN** the turn stops at its next check, the partial answer stays on screen, and the input line accepts text again
