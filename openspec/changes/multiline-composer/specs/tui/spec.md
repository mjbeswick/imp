# tui Specification

## MODIFIED Requirements

### Requirement: The interface is keyboard only

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, and Ctrl+J for a newline, Esc to cancel, Ctrl+R to retry, Ctrl+O to expand the last tool output, Ctrl+T to expand thinking, Ctrl+H to hide thinking, Ctrl+L to clear the view, and Ctrl+C twice to quit.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Start a second line
- **WHEN** the user types a line and presses Shift+Enter
- **THEN** the caret moves to a new row, nothing is sent, and the first line stays in the composer

#### Scenario: Terminals without modifier reporting
- **WHEN** the terminal never reports Shift+Enter separately
- **THEN** Alt+Enter and Ctrl+J still start a new line

#### Scenario: Quit guard
- **WHEN** the user presses Ctrl+C once
- **THEN** the status line asks for a second press, and no second press within two seconds returns to normal

#### Scenario: Cancel a running turn
- **WHEN** the user presses Esc while a turn is streaming or running a tool
- **THEN** the turn stops at its next check, the partial answer stays on screen, and the input line accepts text again

### Requirement: The interface is one chat column

The system SHALL render a scrolling transcript, an input area of one or more rows, and one dim status line at the bottom, and SHALL NOT use panes, borders, tabs, or widgets.

#### Scenario: Quiet layout
- **WHEN** the session is idle
- **THEN** the screen shows the transcript, an empty input row, and one status line

#### Scenario: Long transcript
- **WHEN** the transcript exceeds the screen height
- **THEN** it scrolls within the same column and no second pane appears

#### Scenario: Taller composer
- **WHEN** the composer holds three rows
- **THEN** the transcript gives up two rows, the status line stays last, and the layout is otherwise unchanged

### Requirement: Input history is kept and recalled

The system SHALL persist submitted prompts to a history file, SHALL recall them with Up and Down when the caret is on the first or last composer row, SHALL move the caret between composer rows before leaving the composer, SHALL skip consecutive duplicates, and SHALL cap the file at 1,000 entries.

#### Scenario: Recall
- **WHEN** the user presses Up with the caret on the first composer row
- **THEN** the previous prompt appears in the composer and can be sent unchanged

#### Scenario: Across sessions
- **WHEN** a new session starts
- **THEN** the previous session's prompts are still recallable

#### Scenario: Duplicate suppression
- **WHEN** the same prompt is sent twice in a row
- **THEN** it appears once in history

#### Scenario: Rows before history
- **WHEN** the composer holds three rows and the caret is on the third
- **THEN** Up moves the caret to the second row and the recalled history entry does not appear

## ADDED Requirements

### Requirement: The composer grows with its text

The system SHALL draw the composer across one row per newline in the input, SHALL prefix the first row with `> ` and every other row with two spaces, SHALL keep the caret visible on its row, SHALL grow to at most eight rows or half the screen, whichever is smaller, and SHALL never wrap a composer row past the terminal width.

#### Scenario: Two rows
- **WHEN** the input holds `line one` and `line two` on separate rows
- **THEN** both rows are visible, the second row starts with two spaces, and the status line stays the last row

#### Scenario: Caret on its row
- **WHEN** the caret sits in the middle of the second row
- **THEN** the caret block is drawn there and no other row carries it

#### Scenario: At the cap
- **WHEN** the input holds more rows than the composer may show
- **THEN** the composer shows only the last rows and keeps the caret row visible

#### Scenario: Wide row
- **WHEN** one composer row is wider than the terminal
- **THEN** the row is truncated with a leading ellipsis and never wraps

#### Scenario: After sending
- **WHEN** a multi-row prompt is sent
- **THEN** the composer returns to one empty row and the transcript shows the prompt
