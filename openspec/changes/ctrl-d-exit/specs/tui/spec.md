## REMOVED Requirements

### Requirement: The interface is keyboard first

## ADDED Requirements

### Requirement: The interface is keyboard first with Ctrl+D always exiting

The system SHALL provide Enter to send, or mid-turn to steer or queue, Ctrl+Q to switch that, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+R to retry, Ctrl+T to toggle thinking, Ctrl+E to expand tool rows, Ctrl+L to clear the view, PgUp/PgDn to scroll, Ctrl+C to decline, close, clear, or stop before it quits, and Ctrl+D to exit at once, SHALL decode those keys from control bytes or `CSI` sequences, and SHALL NOT type escape parameters as text.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Clear a draft
- **WHEN** the session is idle, the composer holds text, and the user presses Ctrl+C
- **THEN** the text is deleted, nothing is sent, and the app stays open

#### Scenario: Quit guard
- **WHEN** the composer is empty, nothing runs, and the user presses Ctrl+C once
- **THEN** the app stays open and asks for a second press

#### Scenario: Draft mid-turn
- **WHEN** a turn is streaming, the composer holds text, and the user presses Ctrl+C
- **THEN** the draft is cleared and the turn keeps running

#### Scenario: Cancel then exit
- **WHEN** a turn is streaming, the composer is empty, and the user presses Ctrl+C twice
- **THEN** the first press stops the turn and the second asks before exiting

#### Scenario: Picker open
- **WHEN** a picker is open, the composer is empty, and the user presses Ctrl+C
- **THEN** the picker closes and the app stays open

#### Scenario: Picker open with text
- **WHEN** a picker is open and the composer holds text
- **THEN** Ctrl+C closes the picker, clears the text, and the app stays open

#### Scenario: Confirmation open
- **WHEN** a command confirmation or an `ask` question is open and the user presses Ctrl+C
- **THEN** it is declined and the app stays open

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
- **THEN** it follows the same rule as a raw 0x03

#### Scenario: Page keys
- **WHEN** the terminal sends `CSI 5~` or `CSI 6~`
- **THEN** the transcript pages up or down and no `5~` or `6~` text reaches the composer

#### Scenario: Expand moved
- **WHEN** the user presses Ctrl+E after a tool call
- **THEN** the latest turn's tool rows expand, and Ctrl+O goes to the next waiting session instead

#### Scenario: Newline keys
- **WHEN** the user presses Alt+Enter or Ctrl+J, or the terminal reports Shift+Enter
- **THEN** a newline is inserted and nothing is sent

#### Scenario: Ctrl+C stops a turn
- **WHEN** a turn is running, the draft is empty, and the user presses Ctrl+C
- **THEN** the turn stops and imp stays open

#### Scenario: Ctrl+C clears a draft first
- **WHEN** a turn is running and the draft holds text
- **THEN** Ctrl+C clears the draft and the turn goes on

#### Scenario: Ctrl+C on a prompt
- **WHEN** a confirmation is open and the user presses Ctrl+C
- **THEN** the command does not run, the turn stops, and imp stays open

#### Scenario: Quitting takes two
- **WHEN** nothing runs, the draft is empty, and the user presses Ctrl+C
- **THEN** a line says `Ctrl+C again to quit`, and a second Ctrl+C within two seconds exits

#### Scenario: Ctrl+D exits with a draft
- **WHEN** the composer holds text and the user presses Ctrl+D
- **THEN** the app exits at once and the draft is discarded

#### Scenario: Ctrl+D quits
- **WHEN** the composer is empty and the user presses Ctrl+D
- **THEN** the app exits

#### Scenario: Ctrl+D in a picker
- **WHEN** a picker is open and the user presses Ctrl+D
- **THEN** the app exits and the picker does not run any row action

#### Scenario: Retry key
- **WHEN** the last reply is done and the user presses Ctrl+R
- **THEN** the reply and its tool rows are replaced by a new turn for the same message

#### Scenario: Thinking key
- **WHEN** the user presses Ctrl+T
- **THEN** thinking switches for the next turn, as `/think` does
