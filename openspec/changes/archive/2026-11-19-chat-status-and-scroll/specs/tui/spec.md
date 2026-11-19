# Spec Delta

## MODIFIED Requirements

### Requirement: The interface is keyboard only

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, and Ctrl+J for a newline, Esc to cancel a running turn, Ctrl+R to retry, Ctrl+O to expand the last tool output, Ctrl+T to expand thinking, Ctrl+H to hide thinking, Ctrl+L to clear the view, PgUp and PgDn to scroll the transcript, and Ctrl+C to exit from any state, SHALL decode those keys whether the terminal sends control bytes or disambiguated `CSI` sequences, and SHALL NOT type escape parameters as text.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Quit guard
- **WHEN** the session is idle and the user presses Ctrl+C
- **THEN** the app exits on that press

#### Scenario: Cancel then exit
- **WHEN** a turn is streaming and the user presses Ctrl+C
- **THEN** the app exits on that press and the turn ends with it

#### Scenario: Picker open
- **WHEN** the session picker, agent picker, or an `ask` question is open and the user presses Ctrl+C
- **THEN** the app exits instead of swallowing the key

#### Scenario: Confirmation open
- **WHEN** a command confirmation is open and the user presses Ctrl+C
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
- **THEN** the app exits, exactly as it does for a raw 0x03

#### Scenario: Page keys
- **WHEN** the terminal sends `CSI 5~` or `CSI 6~`
- **THEN** the transcript pages up or down and no `5~` or `6~` text reaches the composer

### Requirement: Output streams as it arrives

The system SHALL render each token within 100 ms of receiving it, keep the newest line visible while following the tail, scroll a page per PgUp or PgDn, hold the viewed rows in place when new output arrives while detached, show the number of unseen rows in the status line, and resume following when PgDn reaches the newest row or the composer is next edited.

#### Scenario: Long answer
- **WHEN** an answer longer than the pane arrives over several seconds
- **THEN** text grows continuously and the pane follows the tail

#### Scenario: Scroll detaches
- **WHEN** the user presses PgUp during streaming
- **THEN** following stops, the viewed rows stay put as more output arrives, and the status line shows a count of unseen rows such as `↑ 12`

#### Scenario: Scroll to the top
- **WHEN** the user keeps pressing PgUp
- **THEN** the view stops at the first transcript row and the unseen count keeps growing

#### Scenario: Resume following
- **WHEN** the user presses PgDn until the newest row is visible, or edits the composer
- **THEN** the view follows the tail again and the unseen count disappears

#### Scenario: Scrolling without streaming
- **WHEN** the user presses PgUp at an idle prompt
- **THEN** earlier rows appear and no request is sent

#### Scenario: Half-written markup
- **WHEN** a stream stops in the middle of a link or a fence
- **THEN** the partial line renders as plain text and no broken escape sequence reaches the terminal

## REMOVED Requirements

### Requirement: The status line reports cost signals

**Reason**: the cache share and the permission mode used width the transcript needed, and five-digit tokens with six-decimal cost were unreadable at a glance.

**Migration**: the status-line requirement below replaces it. Tokens are abbreviated, and cost stays but is shown at two decimals from a running total.

## ADDED Requirements

### Requirement: The status line reports tokens and cost

The system SHALL show the model name, the estimated context tokens, and the running cost of the messages sent in the current process, SHALL abbreviate token counts at or above 1,000 with K and at or above 1,000,000 with M using one decimal, SHALL render cost with two decimals, SHALL NOT show the cache hit share or the permission mode, and SHALL omit a figure it cannot compute rather than guessing.

#### Scenario: Abbreviated tokens
- **WHEN** the estimated context is 3,400 tokens
- **THEN** the status line shows `3.4K tok`, and a context of 1,200,000 shows `1.2M tok`

#### Scenario: Small counts stay exact
- **WHEN** the estimated context is 940 tokens
- **THEN** the status line shows `940 tok`

#### Scenario: Two-decimal cost
- **WHEN** the messages sent in this process cost 0.4246
- **THEN** the status line shows `$0.42`

#### Scenario: Cost accumulates
- **WHEN** a second response costs 0.0531
- **THEN** the status line shows the sum, `$0.48`, not the last response alone

#### Scenario: Removed signals
- **WHEN** any frame is drawn
- **THEN** the status line contains no cache percentage and no permission mode

#### Scenario: Unknown model
- **WHEN** the configured model has no price entry
- **THEN** the status line shows token counts and no cost

### Requirement: The app prints its session id on exit

The system SHALL print the current session id to stderr whenever an exit follows a session being opened, covering the full-screen chat, line mode, and a one-shot prompt, SHALL use the form `imp: resume with imp -r <id>`, and SHALL print it only after the alternate screen, cursor, and cooked input are restored. It SHALL NOT print a session id for commands that open no session, such as `--help`, `--version`, `--list`, `--rename`, `--delete`, `init`, and `prompt`.

#### Scenario: Chat exit
- **WHEN** the user quits the full-screen chat with Ctrl+C
- **THEN** the primary screen shows one line naming the session id that `imp -r` accepts

#### Scenario: Line mode exit
- **WHEN** an interactive `--cli` session ends with `/quit` or end of input
- **THEN** the same resume line is printed

#### Scenario: One-shot exit
- **WHEN** `imp "why is the build failing?"` finishes its single turn
- **THEN** the resume line is printed after the answer

#### Scenario: Piped output stays clean
- **WHEN** stdout is a pipe
- **THEN** the resume line goes to stderr and the piped answer contains no session id

#### Scenario: No session created
- **WHEN** the user runs `imp --list`
- **THEN** no resume line is printed

