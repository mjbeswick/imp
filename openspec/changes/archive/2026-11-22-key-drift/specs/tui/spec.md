## MODIFIED Requirements

### Requirement: The interface is keyboard first

The system SHALL provide Enter to send, or during a turn to steer or queue by the mode, Ctrl+Q to toggle that mode, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+E to expand or collapse the latest turn's tool rows, Ctrl+L to clear the view, PgUp/PgDn to scroll, Ctrl+C to decline an open prompt and stop the turn, else to close an open picker with the draft, else to clear a non-empty draft, else to stop a running turn, else to ask for a second Ctrl+C within two seconds before exiting, and Ctrl+D to exit from an empty draft only, SHALL decode those keys whether the terminal sends control bytes or disambiguated `CSI` sequences, and SHALL NOT type escape parameters as text.

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

#### Scenario: Ctrl+D keeps a draft
- **WHEN** the composer holds text and the user presses Ctrl+D
- **THEN** nothing happens and the draft stays

#### Scenario: Ctrl+D quits
- **WHEN** the composer is empty and the user presses Ctrl+D
- **THEN** the app exits

### Requirement: A keys card lists the chat keys

The system SHALL open a keys card listing the chat keys in the picker overlay when the user types `?` into an empty composer or runs `/keys`, SHALL close it with Esc or `?`, and SHALL show the keys that act in every overlay and confirmation in that view's bottom edge or hint.

#### Scenario: Keys card
- **WHEN** the composer is empty and the user types `?`
- **THEN** the keys card opens and nothing is sent

#### Scenario: Question mark in text
- **WHEN** the composer holds `why` and the user types `?`
- **THEN** the composer reads `why?` and no card opens

#### Scenario: Keys command
- **WHEN** the user runs `/keys`
- **THEN** the keys card opens, and each key with a fallback shows it, such as `Shift+Enter, Alt+Enter, Ctrl+J`
