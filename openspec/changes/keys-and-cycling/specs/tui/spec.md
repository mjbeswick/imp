## MODIFIED Requirements

### Requirement: The interface is keyboard first

The system SHALL provide Enter to send, or during a turn to steer or queue by the mode, Ctrl+Q to toggle that mode, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+R to retry, Ctrl+E to expand or collapse the latest turn's tool rows, Ctrl+T and Ctrl+H to show or hide thinking, Ctrl+L to clear the view, PgUp/PgDn to scroll, Ctrl+C to decline an open prompt and stop the turn, else to stop a running turn, else to clear a non-empty draft, else to ask for a second Ctrl+C within two seconds before exiting, Ctrl+D to exit on an empty draft, SHALL decode those keys whether the terminal sends control bytes or disambiguated `CSI` sequences, and SHALL NOT type escape parameters as text.

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

#### Scenario: Newline keys
- **WHEN** the user presses Alt+Enter or Ctrl+J, or the terminal reports Shift+Enter
- **THEN** a newline is inserted and nothing is sent

#### Scenario: Ctrl+C stops a turn
- **WHEN** a turn is running and the user presses Ctrl+C
- **THEN** the turn stops, the draft is kept, and imp stays open

#### Scenario: Ctrl+C on a prompt
- **WHEN** a confirmation is open and the user presses Ctrl+C
- **THEN** the command does not run, the turn stops, and imp stays open

#### Scenario: Quitting takes two
- **WHEN** nothing runs, the draft is empty, and the user presses Ctrl+C
- **THEN** a line says `Ctrl+C again to quit`, and a second Ctrl+C within two seconds exits

### Requirement: Twenty-three slash commands, no more

The system SHALL provide exactly `/help`, `/keys`, `/new`, `/goal`, `/copy`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/mode`, `/next`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/queue`, `/rename`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty-three commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Rename
- **WHEN** the user runs `/rename fix the parser`
- **THEN** the current session carries that title and no request is sent

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

### Requirement: Suggestions are accepted with one key

The system SHALL draw a suggestion greyed out in an empty input with no key hint after it, SHALL fill the input with it on Right, SHALL drop it as soon as the user types, sends, or a turn starts, and SHALL NOT send it on Enter alone.

#### Scenario: Accept
- **WHEN** the input is empty, `run the tests` is suggested, and the user presses Right then Enter
- **THEN** `run the tests` is sent

#### Scenario: Typed over
- **WHEN** a suggestion is shown and the user types `n`
- **THEN** the input holds `n` and the suggestion is gone

#### Scenario: Late arrival
- **WHEN** a suggestion arrives after the user has started typing
- **THEN** it is not shown

#### Scenario: No hint
- **WHEN** `run the tests` is suggested
- **THEN** the input row shows `run the tests` and no `Tab` after it

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
- **THEN** the keys card opens, and each key with a fallback shows it, such as `Alt+Enter, Ctrl+Q`

### Requirement: Help is grouped, aligned, and shared

The system SHALL print help in named sections, SHALL show defaults as `(default: X)`, SHALL describe every subcommand listed under usage, SHALL show only keys that work in the current front-end, SHALL keep chat-only input such as `!cmd` out of the shell examples, and SHALL use the same command table for `--help`, `/help`, and line mode.

#### Scenario: Aligned sections
- **WHEN** the user runs `imp --help`
- **THEN** every flag starts in one column and every description in every section starts in one shared column

#### Scenario: Long entry
- **WHEN** an example command is longer than the column cap
- **THEN** it keeps its own row and its description starts on the next row at the shared column

#### Scenario: Wrapped description
- **WHEN** a flag description is wider than the terminal
- **THEN** it wraps under the description column and the flag row stays whole

#### Scenario: Command sections
- **WHEN** the user runs `/help`
- **THEN** the commands appear in sections with aligned rows, no keys section, and a last line pointing to `?` and `/keys`

#### Scenario: Narrow terminal
- **WHEN** the width is 60 columns
- **THEN** no help row is wider than 60 and no flag name is cut

#### Scenario: Hackberry width
- **WHEN** the width is 54 or 40 columns
- **THEN** no help row is wider than the screen, and at 40 each description sits under its cell

#### Scenario: Default row
- **WHEN** a flag has a default
- **THEN** `(default: X)` is on its own row at the description column and is never split

#### Scenario: Example note
- **WHEN** `imp --help` lists `imp "why is the build failing?"`
- **THEN** the row above it reads `# one request, then exit`

#### Scenario: Colour
- **WHEN** `imp --help` prints to a pipe or with `NO_COLOR` set
- **THEN** it contains no escape sequences

#### Scenario: Flag help
- **WHEN** the user runs `imp --help` in an 80-column terminal
- **THEN** no line exceeds 80 columns, every flag appears once, and defaults are shown

#### Scenario: Same commands everywhere
- **WHEN** help is printed in the chat, line mode, or with `--help`
- **THEN** the same twenty-three commands and the same flags are named in each

#### Scenario: Keys per front-end
- **WHEN** the user runs `/help` in line mode
- **THEN** no picker, scroll, or Tab key is listed

#### Scenario: Subcommands
- **WHEN** the user runs `imp --help`
- **THEN** `init` and `prompt` each carry a one-line description

#### Scenario: Shell examples
- **WHEN** the user runs `imp --help`
- **THEN** every example line is a command the shell accepts, and `!cmd` is described as chat input

#### Scenario: Unknown option
- **WHEN** the user runs `imp --bogus`
- **THEN** stderr names the option and points to `imp --help`

#### Scenario: Unknown command
- **WHEN** the user types `/compac`
- **THEN** the reply suggests `/compact`

## ADDED Requirements

### Requirement: Tab cycles the agent and Shift+Tab the mode

The system SHALL cycle the permission mode through `normal`, `strict`, and `yolo` on Shift+Tab, applying it at once, SHALL cycle agent profiles on Tab when the draft is empty and no picker is open, applying a change made during a turn when that turn ends, SHALL write one transcript line for each change, and SHALL show the agent and the mode in the status line.

#### Scenario: Mode
- **WHEN** the mode is `strict` and the user presses Shift+Tab
- **THEN** the mode is `yolo`, the prompt and rule turn to the error colour, and one line says so

#### Scenario: Agent
- **WHEN** two agents exist, the draft is empty, and the user presses Tab
- **THEN** the other agent is active for the next message

#### Scenario: Mid-turn agent
- **WHEN** a turn is running and the user presses Tab
- **THEN** the status line shows the coming agent and the switch happens when the turn ends

#### Scenario: Text in the draft
- **WHEN** the draft holds `fix` and the user presses Tab
- **THEN** nothing changes
