## MODIFIED Requirements

### Requirement: The interface is keyboard first

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+R to retry, Ctrl+E to expand or collapse the latest turn's tool rows, Ctrl+T and Ctrl+H to show or hide thinking, Ctrl+L to clear the view, PgUp/PgDn to scroll, and Ctrl+C to clear a non-empty composer or exit when it is empty, SHALL decode those keys whether the terminal sends control bytes or disambiguated `CSI` sequences, and SHALL NOT type escape parameters as text.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Clear a draft
- **WHEN** the session is idle, the composer holds text, and the user presses Ctrl+C
- **THEN** the text is deleted, nothing is sent, and the app stays open

#### Scenario: Quit guard
- **WHEN** the composer is empty and the user presses Ctrl+C
- **THEN** the app exits on that press

#### Scenario: Draft mid-turn
- **WHEN** a turn is streaming, the composer holds text, and the user presses Ctrl+C
- **THEN** the draft is cleared and the turn keeps running

#### Scenario: Cancel then exit
- **WHEN** a turn is streaming, the composer is empty, and the user presses Ctrl+C
- **THEN** the app exits on that press and the turn ends with it

#### Scenario: Picker open
- **WHEN** a picker is open, the composer is empty, and the user presses Ctrl+C
- **THEN** the app exits instead of swallowing the key

#### Scenario: Picker open with text
- **WHEN** a picker is open and the composer holds text
- **THEN** Ctrl+C closes the picker, clears the text, and the app stays open

#### Scenario: Confirmation open
- **WHEN** a command confirmation or an `ask` question is open and the user presses Ctrl+C
- **THEN** the app exits on that press

#### Scenario: Start a second line
- **WHEN** the user types a line and presses Shift+Enter
- **THEN** the caret moves to a new row, nothing is sent, and the first line stays in the composer

#### Scenario: Terminals without modifier reporting
- **WHEN** the terminal never reports Shift+Enter separately
- **THEN** Alt+Enter and Ctrl+J still start a new line

#### Scenario: Cancel a running turn
- **WHEN** the user presses Esc while a turn is streaming or running a tool
- **THEN** the turn stops at its next check, the partial answer stays on screen, and the input line accepts text again

#### Scenario: Disambiguated control keys
- **WHEN** the terminal reports Ctrl+C as `CSI 99;5u`
- **THEN** it follows the same clear-or-exit rule as a raw 0x03

#### Scenario: Page keys
- **WHEN** the terminal sends `CSI 5~` or `CSI 6~`
- **THEN** the transcript pages up or down and no `5~` or `6~` text reaches the composer

#### Scenario: Expand moved
- **WHEN** the user presses Ctrl+E after a tool call
- **THEN** the latest turn's tool rows expand, and Ctrl+O goes to the next waiting session instead

### Requirement: Tool activity is one line each

The system SHALL render each tool call as one row with what it does, its arguments, and elapsed time, SHALL animate the leading spinner at least every 100 ms while it runs, SHALL replace it with a bold mark when it finishes: `✓` when done, `!` in the accent colour when a command ran and exited non-zero, `✗` in the muted colour when the user declined, and `✗` in the error colour for any other failure, SHALL put a preview on indented lines and wrap to that indent, SHALL show the last three output lines of a non-zero exit with their line breaks kept, SHALL show a decline once, as `declined` on its row, SHALL update the row in place, SHALL charge only the tool's work, SHALL show failures with their reason in the error colour, SHALL expand or collapse every tool row of the latest turn on Ctrl+E and one such row on a click, and SHALL show expanded arguments as `$ <command>` for a shell call and as `key: value` lines otherwise.

#### Scenario: Failed tool
- **WHEN** a command runs and exits with code 1
- **THEN** its row starts with `!` in the accent colour, not `✗`, and still carries the duration

#### Scenario: Bold mark
- **WHEN** a tool row is drawn
- **THEN** the leading mark is bold and the rest of the row keeps its role colour

#### Scenario: Failure detail
- **WHEN** a command exits non-zero and its first output line is `Changes: render-on-change`
- **THEN** that text starts on its own indented line and is never appended after the exit code

#### Scenario: Wrapped summary
- **WHEN** a tool summary is wider than the terminal
- **THEN** every continuation line begins at the row's content indent

#### Scenario: While it runs
- **WHEN** a tool call is running
- **THEN** one row shows the call with an animated spinner, the status line shows no spinner, and the transcript has no second row for it

#### Scenario: Spinner advances
- **WHEN** a call runs for 300 ms
- **THEN** its leading glyph has advanced through at least two frames

#### Scenario: When it finishes
- **WHEN** the call returns
- **THEN** the same row carries its mark, the result summary, and the time the tool took

#### Scenario: Confirmation is not charged
- **WHEN** a confirmation is left open for 50 seconds and the command runs for 5 ms
- **THEN** the row reports milliseconds, not the wait

#### Scenario: Refused call
- **WHEN** a write is refused because the path is outside the session
- **THEN** the row shows `✗` with the reason

#### Scenario: Expand the last call
- **WHEN** the user presses Ctrl+E after a turn with a `✗` row and then a `✓` row
- **THEN** both rows show their full arguments and output, and pressing it again collapses them

#### Scenario: Exit tail
- **WHEN** a command exits 1 after printing ten lines
- **THEN** the last three lines appear under its row, one per line, below a count of the seven before them

#### Scenario: Expanded arguments
- **WHEN** a shell call running `python3 calc.py` is expanded
- **THEN** it shows `$ python3 calc.py` and no JSON

#### Scenario: Click a row
- **WHEN** the mouse is on and the user clicks one tool row of the latest turn
- **THEN** only that row expands

#### Scenario: Declined call
- **WHEN** the user declines a command
- **THEN** its row shows a muted `✗` and `declined`, and no other row repeats the decline

### Requirement: Line-mode tool rows are aligned

The system SHALL print a tool start row, then a finish row carrying the same mark as the chat, `✓`, `!`, or `✗`, and the elapsed time, SHALL put a failure preview on its own indented line, SHALL cap the preview at two lines or 240 characters with a marker when cut, and SHALL separate tool blocks from surrounding text with one blank line.

#### Scenario: Success
- **WHEN** a command finishes successfully
- **THEN** the start row stays and a green `✓` row follows

#### Scenario: Failure
- **WHEN** a command exits non-zero
- **THEN** an accent `!` row follows and the preview starts on its own indented line

#### Scenario: Refusal
- **WHEN** a write is refused
- **THEN** a red `✗` row follows with the reason

#### Scenario: Large preview
- **WHEN** the result has twenty lines
- **THEN** two lines are shown with a note that more was cut

#### Scenario: Spacing
- **WHEN** a tool block follows an answer
- **THEN** exactly one blank line separates them
