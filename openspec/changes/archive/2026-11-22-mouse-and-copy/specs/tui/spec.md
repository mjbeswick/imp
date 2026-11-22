## RENAMED Requirements

- FROM: `### Requirement: The interface is keyboard only`
- TO: `### Requirement: The interface is keyboard first`

- FROM: `### Requirement: Twenty-one slash commands, no more`
- TO: `### Requirement: Twenty-three slash commands, no more`

## MODIFIED Requirements

### Requirement: The interface is keyboard first

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+R to retry, Ctrl+E to expand the last tool output, Ctrl+T and Ctrl+H to show or hide thinking, Ctrl+L to clear the view, Ctrl+G to release the mouse, Ctrl+V to paste the copy buffer, PgUp/PgDn to scroll, and Ctrl+C to clear a non-empty composer or exit when it is empty, SHALL decode those keys whether the terminal sends control bytes or disambiguated `CSI` sequences, SHALL NOT type escape parameters as text, and SHALL give every mouse action a key.

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
- **THEN** the last tool output expands, and Ctrl+O goes to the next waiting session instead

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
- **THEN** keys and commands appear in separate sections with aligned rows

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

### Requirement: Twenty-three slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/goal`, `/copy`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/mode`, `/next`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/stash`, `/rename`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

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

### Requirement: A shell can be opened from the prompt

The system SHALL suspend the full-screen interface, restore cooked input, and run `$SHELL` in the session directory for `/shell`, SHALL restore the interface and redraw when it exits, and SHALL run `$SHELL` directly in line mode.

#### Scenario: Chat shell
- **WHEN** the user runs `/shell` in chat
- **THEN** the normal shell prompt appears and returning to it redraws the chat

#### Scenario: Line mode shell
- **WHEN** the user runs `/shell` in line mode
- **THEN** the shell runs and returns to the prompt

#### Scenario: No shell variable
- **WHEN** `$SHELL` is unset
- **THEN** `/bin/sh` is used

#### Scenario: Session directory
- **WHEN** the user has run `/cd src` and then runs `/shell`
- **THEN** the shell starts in `src`, and exiting it returns to the chat with `shell closed`

### Requirement: The mouse wheel scrolls the transcript

The system SHALL, when `mouse` is on, ask the terminal for SGR mouse reports while the chat is open, SHALL scroll the transcript three rows per wheel step, SHALL resume following when scrolled to the end, SHALL move a picker's highlight one row per step, SHALL hand clicks to the click requirements, and SHALL stop mouse reports on exit, on a crash, and while a shell or attached command has the terminal.

#### Scenario: Scroll up
- **WHEN** the transcript is longer than the screen and the user scrolls the wheel up
- **THEN** the view moves up three rows within imp and the terminal's own scrollback is not shown

#### Scenario: Back to the end
- **WHEN** the user scrolls down past the newest row
- **THEN** the view follows new output again

#### Scenario: Picker
- **WHEN** a picker is open and the user scrolls down
- **THEN** the highlight moves down one row

#### Scenario: Exit
- **WHEN** the chat closes or crashes
- **THEN** mouse reporting is off and the terminal handles the mouse again


## ADDED Requirements

### Requirement: Clicks act like the key they show

The system SHALL, when `mouse` is on, treat a left click on a picker row, a confirm chip, a question option, or a picker footer hint as the key that item shows, and SHALL ignore clicks anywhere else.

#### Scenario: Picker row
- **WHEN** the session picker is open and the user clicks its third row
- **THEN** that session opens, as if it were highlighted and Enter pressed

#### Scenario: Confirm chip
- **WHEN** a command confirmation is open and the user clicks `[y] yes`
- **THEN** the command runs, as if `y` were pressed

#### Scenario: Question option
- **WHEN** an `ask` question is open and the user clicks `[2]`
- **THEN** option two is the answer

#### Scenario: Footer hint
- **WHEN** a picker is open and the user clicks `Esc close`
- **THEN** the picker closes

#### Scenario: Elsewhere
- **WHEN** the user clicks a transcript row with no copy label
- **THEN** nothing changes

#### Scenario: Mouse off
- **WHEN** `mouse=off`
- **THEN** no click is reported and the keys still work

### Requirement: Always takes two clicks

The system SHALL treat a first click on `[a] always` as a request to confirm, marking that chip, and SHALL apply `always` only on a second click on the same chip; any other click or key clears the mark.

#### Scenario: One click
- **WHEN** the user clicks `[a] always` once
- **THEN** the chip is marked and the command has not run

#### Scenario: Two clicks
- **WHEN** the user clicks `[a] always` twice
- **THEN** the command runs and is allowed from then on

#### Scenario: Key still works
- **WHEN** the user presses `a`
- **THEN** `always` applies at once, as before

### Requirement: Replies and code blocks carry a copy label

The system SHALL, when `mouse` is on, draw a dim `⧉` at the right edge of the first row of each assistant reply and each fenced code block, and SHALL copy that reply's markdown source or that block's code when it is clicked.

#### Scenario: Copy a code block
- **WHEN** the user clicks the `⧉` beside a fenced block
- **THEN** the block's code, without its fences, is copied

#### Scenario: Copy a reply
- **WHEN** the user clicks the `⧉` on a reply's first row
- **THEN** the reply's markdown source is copied

#### Scenario: Fits the screen
- **WHEN** a label is drawn at 40 columns
- **THEN** no row is wider than 40 columns

#### Scenario: Mouse off
- **WHEN** `mouse=off`
- **THEN** no label is drawn

### Requirement: Text can be copied with /copy

The system SHALL copy the last assistant reply for `/copy` and the last fenced code block for `/copy code`, SHALL report what was copied, and SHALL report when there is nothing to copy.

#### Scenario: Last reply
- **WHEN** the user runs `/copy` after a 120-character reply
- **THEN** that reply is copied and an info row reads `copied 120 characters`

#### Scenario: Last code block
- **WHEN** the user runs `/copy code`
- **THEN** the code of the newest fenced block is copied

#### Scenario: Nothing yet
- **WHEN** the user runs `/copy` before any reply
- **THEN** an info row reads `nothing to copy` and nothing is sent to the model

### Requirement: A copy reaches imp and the terminal clipboard

The system SHALL keep the latest copied text in a buffer for the chat, SHALL paste it into the composer on Ctrl+V, and SHALL also send it to the terminal clipboard as an OSC 52 sequence.

#### Scenario: Paste
- **WHEN** the user copies a block and presses Ctrl+V
- **THEN** the block's text is inserted at the caret, newlines kept

#### Scenario: Terminal clipboard
- **WHEN** a copy is made over SSH from a terminal that accepts OSC 52
- **THEN** the text is on that machine's clipboard

#### Scenario: Empty buffer
- **WHEN** nothing has been copied and the user presses Ctrl+V
- **THEN** the composer is unchanged

### Requirement: The mouse can be released for terminal selection

The system SHALL, on Ctrl+G with `mouse` on, stop mouse reports and show `mouse released · any key to take it back` in the status line, and SHALL resume mouse reports on the next key, which is then handled as usual.

#### Scenario: Release
- **WHEN** the user presses Ctrl+G
- **THEN** mouse reporting is off and the status line says so

#### Scenario: Take it back
- **WHEN** the mouse is released and the user types `h`
- **THEN** mouse reporting resumes and `h` reaches the composer

#### Scenario: Mouse off
- **WHEN** `mouse=off` and the user presses Ctrl+G
- **THEN** nothing changes
