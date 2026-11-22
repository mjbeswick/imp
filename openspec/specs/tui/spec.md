# tui Specification

## Purpose
Defines the terminal interface: what the user sees while a turn runs, which keys exist, what the status line reports, and how the terminal is left after any exit.

## Requirements

### Requirement: The interface is keyboard first

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+R to retry, Ctrl+E to expand the last tool output, Ctrl+T and Ctrl+H to show or hide thinking, Ctrl+L to clear the view, PgUp/PgDn to scroll, and Ctrl+C to clear a non-empty composer or exit when it is empty, SHALL decode those keys whether the terminal sends control bytes or disambiguated `CSI` sequences, and SHALL NOT type escape parameters as text.

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

### Requirement: Every rendered line fits the terminal

The system SHALL clamp every drawn row to the terminal width, SHALL disable the terminal's automatic wrapping while drawing and restore it after, SHALL erase each row before writing it, SHALL clear everything below the status line, and SHALL begin each row at column one.

#### Scenario: Narrow terminal
- **WHEN** the terminal is 40 columns and the status line is longer
- **THEN** the status is truncated with an ellipsis and no row wraps onto the next

#### Scenario: Shrinking window
- **WHEN** the window shrinks between frames
- **THEN** the next frame has no leftover text from the wider one

#### Scenario: No debris
- **WHEN** a menu closes
- **THEN** the rows it used are erased and the chat pane grows back

#### Scenario: Raw terminal mode
- **WHEN** the TUI renders a frame while terminal output processing is disabled
- **THEN** each row boundary emits carriage return and line feed before the next row

#### Scenario: Empty rows before the input
- **WHEN** a short conversation leaves blank rows above the input
- **THEN** the input prompt begins at column one

### Requirement: Line mode frames each turn

The system SHALL print a prompt only when stdin is a terminal, SHALL start model output on a fresh line, and SHALL NOT emit a prompt or escape sequences when piped.

#### Scenario: Interactive line mode
- **WHEN** the user types a message and presses Enter
- **THEN** the answer begins on the next line, never appended to the typed line

#### Scenario: Piped
- **WHEN** input is a pipe
- **THEN** output contains no `> ` prompt and no escape bytes

#### Scenario: Enter arrives as a bare carriage return
- **WHEN** the terminal sends `\r` without translating it to `\n`
- **THEN** the entry is submitted and no `^M` appears in the input

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

### Requirement: Thinking is collapsed by default

The system SHALL show reasoning content as a single dim line with its token count, expandable inline on request, and SHALL keep the collapsed state across turns.

#### Scenario: Expanded reasoning
- **WHEN** the user expands a thinking line
- **THEN** the full reasoning appears in place, dimmed, and can be collapsed again

### Requirement: Tool activity is one line each

The system SHALL render each tool call as one row with what it does, its arguments, and elapsed time, SHALL animate the leading spinner at least every 100 ms while it runs, SHALL replace it with a bold `✓` or `✗` when it finishes, SHALL put a failure preview on an indented line and wrap to that indent, SHALL update the row in place, SHALL charge only the tool's work, SHALL show failures with their reason in the error colour, and SHALL expand the most recent call on request.

#### Scenario: Failed tool
- **WHEN** a tool returns a non-zero exit code
- **THEN** its row is shown in the error colour and still carries the duration

#### Scenario: Bold mark
- **WHEN** a tool row is drawn
- **THEN** the leading `✓` or `✗` is bold and the rest of the row keeps its role colour

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
- **THEN** the same row carries `✓` or `✗`, the result summary, and the time the tool took

#### Scenario: Confirmation is not charged
- **WHEN** a confirmation is left open for 50 seconds and the command runs for 5 ms
- **THEN** the row reports milliseconds, not the wait

#### Scenario: Refused call
- **WHEN** a write is refused because the path is outside the session
- **THEN** the row shows `✗` with the reason

#### Scenario: Expand the last call
- **WHEN** the user presses Ctrl+O
- **THEN** the most recent call shows its full arguments and output, and pressing it again collapses the row

### Requirement: Two front-ends share one loop

The system SHALL offer a line mode and a full-screen chat mode, SHALL pick the full-screen mode when both stdin and stdout are terminals, SHALL pick the line mode otherwise, and SHALL allow the line mode to be forced with `--cli`. It SHALL NOT provide a flag that forces the full-screen mode.

#### Scenario: Interactive launch
- **WHEN** the user runs `imp` in a terminal
- **THEN** the full-screen chat opens

#### Scenario: Scripted launch
- **WHEN** stdout is a pipe
- **THEN** the line mode runs, printing plain text with no escape sequences

#### Scenario: Forced
- **WHEN** the user passes `--cli` in a terminal
- **THEN** the line mode runs without taking over the screen

#### Scenario: No forced chat
- **WHEN** the user passes `--chat`
- **THEN** it is rejected as an unknown option

#### Scenario: Same behaviour
- **WHEN** the same prompt is sent in either mode
- **THEN** the same tools run and the same history is recorded

### Requirement: The interface redraws only when something changes

The system SHALL render a frame only when a key was handled, an event arrived from the session, a resize was noticed, or a visible state changed, SHALL NOT repaint an idle screen on a timer, and SHALL draw the streaming spinner at no more than ten frames a second.

#### Scenario: Idle screen
- **WHEN** the interface sits at an empty prompt with no keys and no events
- **THEN** it writes nothing to the terminal until something changes

#### Scenario: Spinner
- **WHEN** a turn is streaming and the spinner advances
- **THEN** frames are at least 100 ms apart

#### Scenario: Keystroke still repaints
- **WHEN** the user types a character
- **THEN** the frame is redrawn immediately

### Requirement: The interface is one chat column

The system SHALL render a scrolling transcript, an input area of one or more rows, and one dim status line at the bottom, SHALL NOT use panes, tabs, or widgets, SHALL allow exactly one centred bordered overlay for the picker and no other border, and SHALL tell the model the terminal width, and the height when known, before a turn whenever either has changed since the previous turn.

#### Scenario: Quiet layout
- **WHEN** the session is idle and the transcript is empty
- **THEN** the screen shows the welcome row, the rule, an empty input row, and one status line

#### Scenario: Welcome leaves
- **WHEN** the first message is sent
- **THEN** the welcome row is not drawn again

#### Scenario: Turn spacing
- **WHEN** the transcript holds two user entries
- **THEN** one blank row sits directly above the second and none above the first

#### Scenario: Short terminal
- **WHEN** the terminal has 7 rows
- **THEN** no rule is drawn and the transcript keeps that row

#### Scenario: Long transcript
- **WHEN** the transcript exceeds the screen height
- **THEN** it scrolls within the same column and no second pane appears

#### Scenario: Taller composer
- **WHEN** the composer holds three rows
- **THEN** the transcript gives up two rows, the rule and status line keep their places, and the layout is otherwise unchanged

#### Scenario: Resized window
- **WHEN** the window width changes between turns
- **THEN** the next turn carries one line naming the new width, and later turns at that width carry none

#### Scenario: Taller window
- **WHEN** only the window height changes between turns
- **THEN** the next turn carries one line naming the width and the new height

#### Scenario: Piped output
- **WHEN** input is a pipe with no terminal size
- **THEN** no width line is sent

#### Scenario: One overlay
- **WHEN** the picker is open
- **THEN** it is the only bordered element and the chat column resumes unchanged when it closes

### Requirement: The status line reports tokens and cost

The system SHALL show the model name, the served model string when the API reports one that differs from the configured name, the estimated context tokens, and the running cost of the messages sent in the current process, SHALL abbreviate token counts at or above 1,000 with K and at or above 1,000,000 with M using one decimal, SHALL render cost with two decimals, SHALL NOT show the cache hit share or the permission mode, and SHALL omit a figure it cannot compute rather than guessing.

#### Scenario: Abbreviated tokens
- **WHEN** the estimated context is 3,400 tokens
- **THEN** the status line shows `3.4K tok`, and a context of 1,200,000 shows `1.2M tok`

#### Scenario: Small counts stay exact
- **WHEN** the estimated context is 940 tokens
- **THEN** the status line shows `940 tok`

#### Scenario: Served model differs
- **WHEN** the configured model is `deepseek-flash` and a response reports `deepseek-flash-2026-11`
- **THEN** the status line shows `deepseek-flash (deepseek-flash-2026-11)`

#### Scenario: Served model matches
- **WHEN** the response reports the same string as the configured model, or no response has arrived
- **THEN** the status line shows the configured name alone

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

### Requirement: Confirmation prompts appear inline

The system SHALL ask before a destructive command or a memory write on one row in the input area, naming the verb alone when the tool row names the command, SHALL show each answer as a chip, a bracketed key then its label, SHALL accept `y` or Enter, `n`, and `s` and `a` where offered, SHALL decline and stop the turn on Esc, returning the prompt, SHALL treat `y` and Enter as the default answer, and SHALL wait until answered, with no deadline and no countdown.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

#### Scenario: Ask once
- **WHEN** a confirmation opens for a command
- **THEN** the command appears once, in the tool row above, and the prompt reads `run it?   [y] yes  [n] no  [s] session  [a] always`

#### Scenario: Long command
- **WHEN** the command in a confirmation is longer than the row
- **THEN** the command is cut in the middle in the tool row and the prompt keeps its chips

#### Scenario: Deadline shown
- **WHEN** a confirmation opens
- **THEN** its row shows the question and chips and no seconds left

#### Scenario: Unknown verb
- **WHEN** the action does not start with run, write, save, or delete
- **THEN** the prompt reads `approve?   [y] yes  [n] no`

#### Scenario: Chip colours
- **WHEN** a confirmation opens in a colour terminal
- **THEN** each key letter is bold in the accent colour and the labels are muted

#### Scenario: Attention
- **WHEN** a confirmation opens
- **THEN** the terminal bell rings once and does not ring again for that prompt

#### Scenario: No answer
- **WHEN** a confirmation has been open for ten minutes
- **THEN** it is still open, the command has not run, and the turn is still waiting

#### Scenario: Esc stops the turn
- **WHEN** the user presses Esc on a confirmation
- **THEN** the command does not run, the turn stops, and the composer is ready for the next message

#### Scenario: Enter approves
- **WHEN** the user presses Enter on a confirmation
- **THEN** the command runs as if `y` were pressed

#### Scenario: Interrupt a prompt
- **WHEN** the user presses Ctrl+C while a confirmation is open
- **THEN** the prompt closes, the command does not run, and the turn stops

#### Scenario: Always is not offered
- **WHEN** a confirmation cannot be approved beyond once
- **THEN** the prompt shows only `[y] yes  [n] no`, and `s` and `a` do nothing

### Requirement: Questions are answerable by key or by typing

The system SHALL render an `ask` request as a picker above the input line listing the choices with number keys, SHALL offer a custom-answer row reachable by letter key or arrow, SHALL show the typed answer before sending it, and SHALL leave the question unanswered and stop the turn on Esc.

#### Scenario: Pick by number
- **WHEN** the question offers three choices
- **THEN** pressing 2 selects the second and the answer is sent immediately

#### Scenario: Type an answer
- **WHEN** the user chooses the custom row and types a sentence
- **THEN** that sentence is the answer and it appears in the transcript

#### Scenario: Dismiss
- **WHEN** the user presses Esc on a question
- **THEN** the question closes, the tool result says it was not answered, the turn stops, and the input is ready

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

### Requirement: Slash commands autocomplete

The system SHALL open the picker overlay when the input begins with `/`, SHALL show commands filtered by the word before the first space, SHALL rank prefix matches before substring matches, SHALL show a title and a footer naming Enter and Esc, SHALL accept the highlighted command with Tab or Enter, SHALL open the value picker after a command word and space when that command has values, SHALL seed its filter with the text after the space, and SHALL keep it closed for free-text arguments.

#### Scenario: Narrowing
- **WHEN** the user types `/co`
- **THEN** `/compact` is listed and other commands are filtered out

#### Scenario: Accept
- **WHEN** the user presses Tab
- **THEN** the highlighted command is completed in the input line and the overlay closes

#### Scenario: Run immediately
- **WHEN** the user presses Enter on the highlighted command
- **THEN** the command runs and the input clears

#### Scenario: Dismiss
- **WHEN** the user presses Esc
- **THEN** the overlay closes and the typed text stays

#### Scenario: Values follow the command
- **WHEN** the user types `/theme ` and then `gru`
- **THEN** the theme picker opens with `gru` in the filter and `gruvbox-dark` listed

#### Scenario: Free text
- **WHEN** the user types `/stash remember the tests`
- **THEN** no picker opens and the whole text is stashed on Enter

### Requirement: The interface uses colour with meaning

The system SHALL colour by role rather than by text, SHALL paint inline code bold in the code colour and keep fenced blocks and quotes muted, SHALL detect truecolor, 256-colour, and no-colour terminals, and SHALL honour `NO_COLOR` and a `color=auto|always|never` setting.

#### Scenario: Truecolor terminal
- **WHEN** `COLORTERM` advertises truecolor
- **THEN** roles render as 24-bit RGB

#### Scenario: Plain terminal
- **WHEN** the terminal supports 256 colours only
- **THEN** the same roles render from a 256-colour palette that keeps the same contrast relationships

#### Scenario: No colour
- **WHEN** `NO_COLOR` is set or the setting is `never`
- **THEN** no escape sequences are emitted and the layout is unchanged

#### Scenario: Error stands out
- **WHEN** a tool exits non-zero or a request fails
- **THEN** that line renders in the error role and remains readable without colour

#### Scenario: Inline code is not dim
- **WHEN** a reply contains `` `bash` `` inside a sentence
- **THEN** that span is bold in the code colour, not the muted grey

#### Scenario: Fenced code stays dim
- **WHEN** a reply contains a fenced block
- **THEN** its lines are dim and not bold

### Requirement: Shell commands run from the input line

The system SHALL run any input beginning with `!` as a local shell command instead of sending it to the model, SHALL show the command and its output in the transcript, SHALL show `$ ` as the prompt and a shell status while such a draft is open, and SHALL cap that output at 4 KB with the usual spill rule.

#### Scenario: Run a command
- **WHEN** the user types `!git status` and presses Enter
- **THEN** the command runs in the session directory, its output appears in the transcript, and no request is sent to the model

#### Scenario: Failing command
- **WHEN** the command exits non-zero
- **THEN** the exit code and stderr appear in the error role and the session continues

#### Scenario: Large output
- **WHEN** the command writes 1 MB
- **THEN** the transcript shows a short head and tail plus the spill path

#### Scenario: Shell draft
- **WHEN** the composer holds `!git status`
- **THEN** the prompt reads `$ ` and the status row says the command is local

#### Scenario: Back to the model
- **WHEN** the draft no longer starts with `!`
- **THEN** the prompt returns to `> ` and the status row reports model and cost

### Requirement: Typed commands bypass confirmation but not denial

The system SHALL treat a command typed by the user as already approved, SHALL NOT prompt for it, and SHALL still refuse anything on the deny list in every mode.

#### Scenario: No prompt
- **WHEN** the user types `!npm install`
- **THEN** it runs without a confirmation prompt

#### Scenario: Denied
- **WHEN** the user types `!sudo rm -rf /`
- **THEN** it is refused with a message naming the rule

#### Scenario: Denied in yolo
- **WHEN** the mode is `yolo` and the user types `!rm -rf /`
- **THEN** it is refused with a message naming the rule

### Requirement: Typed commands are visible to the model

The system SHALL append a compact record of each typed command, its exit code, and its capped output as a `system` message at the tail of the conversation, and SHALL NOT rewrite the prefix.

#### Scenario: Follow-up question
- **WHEN** the user runs `!git status` and then asks about the output
- **THEN** the model can answer from the recorded command and output

#### Scenario: Prefix untouched
- **WHEN** a typed command is recorded
- **THEN** the bytes before the conversation tail are unchanged

### Requirement: The terminal is always restored

The system SHALL restore cursor visibility, cooked input mode, and the primary screen on normal exit, panic, SIGINT, SIGTERM, and SIGHUP, and SHALL exit within about a second of such a signal even when the terminal has stopped reading.

#### Scenario: Forced panic
- **WHEN** the process panics while the alternate screen is active
- **THEN** the shell prompt returns usable, with no leftover alternate screen or hidden cursor

#### Scenario: Clean exit
- **WHEN** the user quits the TUI
- **THEN** the terminal input mode matches the mode from before the TUI started, and a line-mode program started afterwards submits on Enter

#### Scenario: Signal during a turn
- **WHEN** the process receives SIGINT or SIGTERM while a turn runs
- **THEN** the primary screen and cooked input mode are restored before it exits

#### Scenario: Linux terminal
- **WHEN** the interface runs on Linux
- **THEN** raw mode is applied with the platform's own `termios` layout, so typed keys reach the composer and the original mode is restored on exit

#### Scenario: Terminal not reading
- **WHEN** the terminal stops reading, as a frozen SSH session does, and the process receives SIGTERM or SIGHUP
- **THEN** it exits within about a second

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

### Requirement: Assistant text renders as markdown

The system SHALL render assistant text as a markdown subset: headings, bold, italic, inline code, fenced code, bullet and numbered lists, blockquotes, horizontal rules, and tables, and SHALL NOT render images.

#### Scenario: Emphasis
- **WHEN** the model writes `**done** in 2 s`
- **THEN** the row shows `done` in bold and the rest unchanged

#### Scenario: Fenced code
- **WHEN** the model writes a fenced block
- **THEN** its lines are shown dim, without fence markers, and without wrapping inside the block

#### Scenario: Lists and quotes
- **WHEN** the model writes a bullet list and a quote
- **THEN** bullets render with a marker and the quote renders dim and prefixed

#### Scenario: Table
- **WHEN** the model writes a markdown table
- **THEN** it is drawn as a boxed table rather than raw pipes

### Requirement: Links are clickable where the terminal supports it

The system SHALL draw `[text](url)` as the link text wrapped in an OSC 8 hyperlink when the terminal supports them, SHALL keep the URL on the row when it does not, SHALL link bare `https://` URLs as well, and SHALL never split the sequence across wrapped rows.

#### Scenario: Link terminal
- **WHEN** the terminal advertises hyperlink support and the model writes `[docs](https://example.com/x)`
- **THEN** the row shows `docs` inside an OSC 8 sequence for that URL

#### Scenario: Plain terminal
- **WHEN** the terminal does not advertise support
- **THEN** the row shows `docs (https://example.com/x)`

#### Scenario: Wrapped link
- **WHEN** a link's text wraps onto a second row
- **THEN** each row carries its own complete OSC 8 sequence

#### Scenario: Off
- **WHEN** `links=never`
- **THEN** no OSC 8 sequence is emitted and links render as text with the URL

### Requirement: The session root can be inspected and changed

The system SHALL print the session root for `/cwd`, SHALL change it for `/cd <path>`, SHALL open a directory browser rooted at the session directory for `/cd` with no path in the chat interface, SHALL expand a leading `~`, SHALL resolve a relative path against the current root, SHALL refuse a missing path or a file without changing anything, SHALL keep the conversation, SHALL reload instructions, skills, memory, and the system prompt, and SHALL report the new root and the cache reset.

#### Scenario: Show the root
- **WHEN** the user runs `/cwd`
- **THEN** the transcript shows the absolute path the session is rooted at

#### Scenario: Change the root
- **WHEN** the user runs `/cd /Users/michael/Projects/imp`
- **THEN** the transcript names the new root and says the prompt cache was reset

#### Scenario: Tilde
- **WHEN** the user runs `/cd ~/Projects`
- **THEN** the root becomes the home directory joined with `Projects`

#### Scenario: Relative path
- **WHEN** the user runs `/cd ..` from `~/Projects/imp`
- **THEN** the root becomes `~/Projects`

#### Scenario: Missing path
- **WHEN** the user runs `/cd /nope`
- **THEN** the transcript reports that the directory does not exist and the root is unchanged

#### Scenario: A file
- **WHEN** the user runs `/cd Cargo.toml`
- **THEN** the transcript reports that the path is not a directory and the root is unchanged

#### Scenario: Bare command
- **WHEN** the user runs `/cd` with no path in the chat interface
- **THEN** a directory browser opens at the session root

#### Scenario: Line mode bare command
- **WHEN** the user runs `/cd` with no path in line mode
- **THEN** the usage `cd <path>` prints and nothing changes

#### Scenario: History kept
- **WHEN** the root changes after several turns
- **THEN** the conversation is still present and the next request carries the new instructions

#### Scenario: No model request
- **WHEN** either command runs
- **THEN** nothing is sent to the model

### Requirement: Verbosity controls tool detail

The system SHALL keep a per-session verbosity of `low`, `medium`, or `high`, starting from the config's `verbosity` and defaulting to `medium`, SHALL change it with `/verbosity <level>` without writing the config, SHALL show one line per step at `low`, add a short result at `medium`, and add each call's arguments and capped output at `high` in both front-ends, and SHALL apply the level to later calls.

#### Scenario: Default
- **WHEN** a session starts and the config sets no verbosity
- **THEN** tool calls render as one summary line each, followed by a short result such as `→ 200 json · 12 KB`

#### Scenario: High
- **WHEN** the user runs `/verbosity high` and the next tool call runs
- **THEN** that call shows its arguments and capped output in the transcript

#### Scenario: Back to low
- **WHEN** the user runs `/verbosity low`
- **THEN** later calls return to one summary line without a result and earlier rows stay as they were drawn

#### Scenario: Fetch label
- **WHEN** the model fetches a URL with a long query string
- **THEN** the row names the host and path without the query

#### Scenario: Line mode
- **WHEN** the user runs `/verbosity high` in line mode
- **THEN** each following call prints its arguments and capped output as plain text

#### Scenario: No argument
- **WHEN** the user runs `/verbosity` with no level in the chat interface
- **THEN** the picker lists `low`, `medium`, and `high` with the current level highlighted and choosing one applies it

#### Scenario: Line mode no argument
- **WHEN** the user runs `/verbosity` with no level in line mode
- **THEN** the transcript reports the current level and no picker opens

#### Scenario: Unknown level
- **WHEN** the user runs `/verbosity loud`
- **THEN** the transcript refuses the value and names `low`, `medium`, and `high`

### Requirement: Line mode uses the role palette

The system SHALL paint the prompt, tool rows, errors, and information lines with the active theme when stdout is a terminal and colour is enabled, SHALL emit no escape sequences when stdout is not a terminal or colour is off, and SHALL keep the same text and layout in both cases.

#### Scenario: Colour terminal
- **WHEN** line mode runs in a colour terminal
- **THEN** tool rows, errors, and info lines use their theme colours

#### Scenario: Piped
- **WHEN** stdout is a pipe
- **THEN** the output contains no escape byte and the same words

#### Scenario: Colour off
- **WHEN** `NO_COLOR` is set or `color=never`
- **THEN** the text stays plain even in a terminal

### Requirement: Line-mode tool rows are aligned

The system SHALL print a tool start row, then a finish row carrying `✓` or `✗` and the elapsed time, SHALL put a failure preview on its own indented line, SHALL cap the preview at two lines or 240 characters with a marker when cut, and SHALL separate tool blocks from surrounding text with one blank line.

#### Scenario: Success
- **WHEN** a command finishes successfully
- **THEN** the start row stays and a green `✓` row follows

#### Scenario: Failure
- **WHEN** a command exits non-zero
- **THEN** a red `✗` row follows and the preview starts on its own indented line

#### Scenario: Large preview
- **WHEN** the result has twenty lines
- **THEN** two lines are shown with a note that more was cut

#### Scenario: Spacing
- **WHEN** a tool block follows an answer
- **THEN** exactly one blank line separates them

### Requirement: Themes are selectable

The system SHALL provide `default`, `default-light`, `solarized-dark`, `solarized-light`, `gruvbox-dark`, `gruvbox-light`, `catppuccin-mocha`, `catppuccin-latte`, `tokyo-night`, `nord`, and `mono`, SHALL apply the active theme to both front-ends, SHALL switch with `/theme <name>`, SHALL reject an unknown name and list the valid ones, and SHALL save a switch and load it at startup. `mono` SHALL force colour off.

#### Scenario: List
- **WHEN** the user runs `/theme`
- **THEN** the eleven names appear with the active one marked

#### Scenario: Switch
- **WHEN** the user runs `/theme gruvbox-dark`
- **THEN** the next frame and the next line-mode rows use that palette

#### Scenario: Mono
- **WHEN** the user runs `/theme mono`
- **THEN** no colour is emitted even in a colour terminal

#### Scenario: Unknown
- **WHEN** the user runs `/theme neon`
- **THEN** the transcript refuses it and lists the valid names

#### Scenario: Persistence
- **WHEN** the user switches to `solarized-light` and restarts
- **THEN** the session starts in `solarized-light`

#### Scenario: Bad saved value
- **WHEN** the state file names a theme that does not exist
- **THEN** the session starts in `default` or `default-light` by the background guess

### Requirement: File changes show a diff

After a successful `edit` or `write`, the system SHALL draw the changed lines under the tool row, SHALL mark removed lines in the delete colour and added lines in the add colour, SHALL show at most 12 lines by default with a count of the rest, SHALL show the complete diff when the row is expanded with Ctrl+O, and SHALL keep the diff out of the tool result sent to the model.

#### Scenario: Edit diff
- **WHEN** an edit replaces two lines with three
- **THEN** the row shows the two removed lines in red and the three added lines in green

#### Scenario: New file
- **WHEN** `write` creates a file with 20 lines
- **THEN** the preview shows the first 12 as additions and says 8 more follow

#### Scenario: Overwrite
- **WHEN** `write` replaces an existing file
- **THEN** the diff shows the written content as added lines

#### Scenario: Expand
- **WHEN** the user presses Ctrl+O after an edit
- **THEN** the full diff appears, not only the 12-line preview

#### Scenario: No colour
- **WHEN** colour is off
- **THEN** removed lines begin with `-` and added lines with `+`, and the layout is unchanged

#### Scenario: Model result unchanged
- **WHEN** an edit succeeds
- **THEN** the tool result the model receives is still the short `edited <path>` sentence

#### Scenario: Failure
- **WHEN** an edit is refused because a match failed
- **THEN** no diff is drawn and the existing error preview is shown

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

The system SHALL suspend the full-screen interface, restore cooked input, and run `$SHELL` in the session directory for `/shell`, SHALL restore the interface and redraw when it exits, SHALL NOT read keys or draw frames while the shell holds the terminal, and SHALL run `$SHELL` directly in line mode.

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

#### Scenario: The user's own shell survives
- **WHEN** imp runs from an interactive shell and the user opens `/shell`, types a command, and exits
- **THEN** the command's output shows, imp redraws in full and takes keys, and quitting imp returns to the user's shell

#### Scenario: Keys go to the shell
- **WHEN** the user types twenty commands quickly in `/shell`
- **THEN** all twenty run in the shell and none reaches imp

#### Scenario: No frames over the shell
- **WHEN** `/shell` is open for several seconds
- **THEN** imp draws nothing on the shell's screen

### Requirement: The session model can be chosen

The system SHALL set the session model for `/model <name>`, SHALL open a picker for bare `/model` listing the provider's models, the current model, the provider default, and recents, SHALL fetch that list from the provider and fall back to a built-in catalog when the request fails, SHALL record a chosen model in recents, and SHALL NOT rewrite the config file.

#### Scenario: Direct
- **WHEN** the user runs `/model deepseek-v4-pro`
- **THEN** the next request uses that model

#### Scenario: Picker
- **WHEN** the user runs `/model` with no argument
- **THEN** the picker lists the provider's models, the current model, the default, and recents

#### Scenario: Live list
- **WHEN** the provider reports four models and `/model` opens
- **THEN** all four appear, not only the current model

#### Scenario: Fetch fails
- **WHEN** the models endpoint is unreachable or rejects the key
- **THEN** the picker keeps the catalog entries and the current model

#### Scenario: Filter
- **WHEN** the user types `flash` in the model picker
- **THEN** only names containing `flash` remain, from either source

#### Scenario: Recents
- **WHEN** a model is chosen
- **THEN** it appears at the front of the next picker

### Requirement: Provider profiles can be managed

The system SHALL list built-in and custom providers for bare `/provider`, SHALL support `add`, `edit`, `delete`, and `use`, SHALL store custom profiles in a managed section of the config file while preserving every other line, SHALL require a base URL, model, and dialect for a profile, and SHALL refuse a delete that leaves the active provider missing.

#### Scenario: Add
- **WHEN** the user runs `/provider add work` and supplies base URL, model, and dialect
- **THEN** the config gains a `provider.work.*` block and `work` lists next time

#### Scenario: Edit
- **WHEN** the user edits `work`
- **THEN** only its managed lines change and comments survive

#### Scenario: Delete
- **WHEN** the user deletes a profile that is not active
- **THEN** its lines are removed

#### Scenario: Delete active
- **WHEN** the user deletes the active profile
- **THEN** the command refuses and asks them to switch first

#### Scenario: Use
- **WHEN** the user runs `/provider use work`
- **THEN** the active provider becomes `work` for the session

### Requirement: Messages can be stashed for the session

The system SHALL keep a session-bound stash of messages, SHALL add text with `/stash <text>`, SHALL list entries numbered for bare `/stash`, SHALL load entry `n` into the composer and remove it for `pop n`, SHALL remove it without loading for `delete n`, SHALL load all entries joined by blank lines for `pop all`, and SHALL NOT send a request to the model.

#### Scenario: Add while busy
- **WHEN** the agent is streaming and the user runs `/stash check the tests`
- **THEN** the text is stored and the turn is not interrupted

#### Scenario: List
- **WHEN** the user runs `/stash`
- **THEN** each entry appears with its number

#### Scenario: Pop into composer
- **WHEN** the user runs `/stash pop 2`
- **THEN** entry 2 is removed and its text sits in the composer, ready to edit

#### Scenario: Pop all
- **WHEN** the user runs `/stash pop all`
- **THEN** every entry is removed and joined into the composer

#### Scenario: Delete
- **WHEN** the user runs `/stash delete 1`
- **THEN** entry 1 is removed and the composer is unchanged

### Requirement: The status line has no spinner

The system SHALL start the status line with the model name and SHALL NOT draw a
spinner, animation frame, or activity glyph there, whether the turn is idle,
streaming text, or running a tool.

#### Scenario: Idle
- **WHEN** no turn is running
- **THEN** the status line begins with the model name

#### Scenario: Streaming text
- **WHEN** the model is streaming an answer
- **THEN** the status line is unchanged apart from its token and cost figures

#### Scenario: Tool running
- **WHEN** a tool row is animating
- **THEN** the status line shows no matching glyph

#### Scenario: After the turn
- **WHEN** the turn ends
- **THEN** the status line keeps the same leading model name

### Requirement: Autocomplete rows show a description

The system SHALL render every command row as two columns, SHALL put the command name in the left column and its one-line description in the right column, SHALL align the right column across rows, SHALL never truncate the command name, SHALL clip a description that does not fit, SHALL apply the selection highlight across both columns, and SHALL match the filter against the command name only.

#### Scenario: Two columns
- **WHEN** the user types `/`
- **THEN** `/compact` sits on the left and `compact the conversation now` on the right

#### Scenario: Alignment
- **WHEN** rows of different name lengths are listed
- **THEN** every description starts at the same column

#### Scenario: Narrow terminal
- **WHEN** the box is too narrow for a description
- **THEN** the description is cut and the command name stays whole

#### Scenario: Filter
- **WHEN** the user types `/co`
- **THEN** matching uses the command name, not words in its description

### Requirement: Tool rows share one indent and wrap their content

The system SHALL start every tool row with its mark in column one, SHALL place the summary after one space, SHALL indent wrapped continuations, failure previews, questions, and options by two columns, SHALL keep long summaries whole and wrap them to the terminal width, and SHALL NOT truncate a summary or preview to one line for width.

#### Scenario: Success and failure align
- **WHEN** a successful row and a failed row sit next to each other
- **THEN** both marks are in column one and both summaries start in column three

#### Scenario: Long command
- **WHEN** a bash command is 300 characters wide
- **THEN** it wraps across rows at the two-column indent, and the duration stays at the end of the summary

#### Scenario: Failure preview
- **WHEN** a command times out and its reason is longer than the terminal
- **THEN** the reason wraps at the two-column indent instead of being clipped

#### Scenario: Ask question
- **WHEN** an ask row carries a question and options wider than the terminal
- **THEN** the question and each option wrap beneath the action at the two-column indent

#### Scenario: No width ellipsis
- **WHEN** a row is wider than the terminal
- **THEN** it wraps, and the only ellipsis allowed marks content the model itself shortened

### Requirement: Yolo applies when the command runs

The system SHALL apply `/mode <name>` to the shared mode before its worker command is handled, SHALL honour the new mode for the rest of the running turn, SHALL keep it for the session only without saving it, SHALL name the new mode in the transcript, SHALL report an unknown name as an error listing `normal`, `strict`, and `yolo`, and SHALL keep the deny list and sudo gate in force.

#### Scenario: Enabled mid-turn
- **WHEN** a confirmation would appear and the user has just run `/mode yolo`
- **THEN** the call runs without a prompt

#### Scenario: Disabled
- **WHEN** the user runs `/mode normal`
- **THEN** later calls ask as before

#### Scenario: No double toggle
- **WHEN** `/mode` commands are queued while a turn runs
- **THEN** each carries its target mode and the last one wins

#### Scenario: Hard stops
- **WHEN** the mode is yolo and a command is on the deny list
- **THEN** it is still refused

#### Scenario: Strict mid-session
- **WHEN** the user runs `/mode strict`
- **THEN** every later command asks, and the config file is unchanged

#### Scenario: Unknown mode
- **WHEN** the user runs `/mode wild`
- **THEN** an error lists `normal`, `strict`, and `yolo` and the mode is unchanged

### Requirement: The status line shows the session directory

The system SHALL draw the session directory at the right end of the status line, SHALL shorten the home directory to `~`, SHALL keep at least two spaces between it and the left figures, and SHALL omit it rather than cut the model name or figures when the row is too narrow.

#### Scenario: Wide terminal
- **WHEN** the terminal is 100 columns and the root is the home directory's `Projects/imp`
- **THEN** the status line ends with `~/Projects/imp` in the last column

#### Scenario: Narrow terminal
- **WHEN** the terminal is 40 columns
- **THEN** the directory is absent and the model name is whole

### Requirement: Tool durations are right-aligned

The system SHALL draw a finished tool row's duration in the thinking role, right-aligned on the row's last line with at least two spaces before it, SHALL write durations under one second as whole milliseconds with `ms` and longer ones in seconds with one decimal and `s`, and SHALL move the duration to its own indented row rather than cut it when the line has no room.

#### Scenario: Fast call
- **WHEN** a call takes 5 ms
- **THEN** its row ends with `5ms` in the last column

#### Scenario: Slow call
- **WHEN** a call takes 1400 ms
- **THEN** its row ends with `1.4s`

#### Scenario: Full row
- **WHEN** the summary's last line leaves fewer than two spaces before the duration
- **THEN** the duration is drawn right-aligned on the next row

### Requirement: Pending prompts stand out

The system SHALL paint an open confirmation, question, or input prompt in the accent role behind a `▌` prefix, SHALL paint the rule above it in the accent role while it is open, SHALL draw a text prompt's caret the same way as the composer's, and SHALL keep a confirmation or text prompt to one row, while an `ask` question takes a row for the question, one per choice, and one for the keys, each wrapped at the screen width.

#### Scenario: Confirmation
- **WHEN** a confirmation opens in a colour terminal
- **THEN** its row starts with `▌` in the accent colour and the rule above it is in the accent colour

#### Scenario: No colour
- **WHEN** colour is off and a confirmation opens
- **THEN** the row starts with `▌` and no escape sequence is emitted for colour

#### Scenario: Text prompt caret
- **WHEN** `/rename` asks for a title
- **THEN** the caret is the composer's block caret, not an underscore

#### Scenario: Long choices
- **WHEN** an ask question's choices together are wider than the terminal
- **THEN** each choice starts its own row with its number key, wraps instead of being cut, and the question stays above them

### Requirement: Value commands share one form

The system SHALL treat `/agent`, `/model`, `/theme`, `/verbosity`, `/provider`, and `/sessions` as value commands: bare, the chat SHALL open the picker on the current entry and line mode SHALL print aligned rows with `current` beside the active one, except `/sessions`, which lists newest first; a typed value SHALL apply directly in both front-ends, except `/provider` subcommands, which run only in the chat; and `/sessions <id>` SHALL resume that session.

#### Scenario: Line-mode values
- **WHEN** the user runs `/theme` in line mode
- **THEN** each theme prints on its own row and the active one has `current` beside it

#### Scenario: Resume by id
- **WHEN** the user runs `/sessions 51280befc`
- **THEN** that session is resumed as if picked

### Requirement: Tab completes slash commands

The system SHALL complete the highlighted command on Tab in the command picker, SHALL then open its picker when it is a value command, SHALL leave `/{name} ` in the composer when it takes free text, and SHALL leave `/{name}` unrun when it takes no argument.

#### Scenario: Tab opens values
- **WHEN** the user types `/the` and presses Tab
- **THEN** the composer holds `/theme ` and the theme picker is open with the current theme highlighted

#### Scenario: Tab on free text
- **WHEN** the user types `/ren` and presses Tab
- **THEN** the composer holds `/rename ` and no picker is open

#### Scenario: Tab on a bare command
- **WHEN** the user types `/comp` and presses Tab
- **THEN** the composer holds `/compact` and nothing runs until Enter

### Requirement: Command replies are consistent

The system SHALL report an unknown command or value in the error role with the nearest command or the valid values, and SHALL confirm a setting change as `{setting}: {value}` in both front-ends.

#### Scenario: Unknown value
- **WHEN** the user runs `/verbosity loud` or `/nope`
- **THEN** the reply is in the error role and names the valid values or the nearest command

#### Scenario: Toggle feedback
- **WHEN** the user runs `/think`
- **THEN** both front-ends reply `thinking: on` or `thinking: off`

### Requirement: The chat column is spaced

The system SHALL separate the transcript from the input area with one dim horizontal rule when the terminal has at least 8 rows, SHALL leave one blank row before every user entry except the first, and SHALL show one dim welcome row naming the app and the `/` hint while the transcript is empty, leaving the directory to the status line.

#### Scenario: Rule and gap
- **WHEN** a 24-row screen holds two user entries
- **THEN** a rule sits directly above the input row and one blank row sits above the second entry

#### Scenario: Welcome row
- **WHEN** the transcript is empty
- **THEN** the row above the rule reads `imp · / for commands`

### Requirement: Help shares one description column

The system SHALL align every description in one help output to one column two spaces after the longest left entry in any section, capped at 26 columns, SHALL start the description on the next row at that column when an entry is longer than the cap, SHALL wrap a long description under the same column, and SHALL keep every row inside the requested width.

#### Scenario: One column
- **WHEN** the user runs `imp --help` at 60 columns
- **THEN** every description starts in the same column and no row is wider than 60

### Requirement: Yolo shows in colour

The system SHALL paint the composer prompt glyph and the rule above the composer in the error colour while the live mode is `yolo`, SHALL paint the line-mode prompt the same way, SHALL leave the status line text unchanged, and SHALL emit nothing extra when colour is off.

#### Scenario: Yolo on
- **WHEN** the user runs `/mode yolo` in a colour terminal
- **THEN** the next frame paints `> ` and the rule in the error colour

#### Scenario: Back to normal
- **WHEN** the user runs `/mode normal`
- **THEN** the glyph and rule return to their usual colours

#### Scenario: No colour
- **WHEN** colour is off and the mode is `yolo`
- **THEN** the frame is identical to the `normal` frame

### Requirement: Eleven roles cover the screen

The system SHALL give every theme eleven roles, `text`, `user`, `muted`, `border`, `accent`, `tool`, `success`, `error`, `code`, `link`, and `select`, SHALL paint rules and the picker frame in `border`, `✓` marks and added lines in `success`, `✗` marks and removed lines in `error`, links in `link`, and the highlighted picker row on the `select` background.

#### Scenario: Picker highlight
- **WHEN** a picker is open in a colour terminal
- **THEN** the highlighted row has the `select` background across the inner width

#### Scenario: Marks
- **WHEN** a tool call succeeds
- **THEN** its `✓` is bold in the success colour and the summary is in the tool colour

### Requirement: The default follows the terminal background

The system SHALL, when no theme is saved, choose `default-light` for a light terminal background and `default` otherwise, SHALL read `COLORFGBG` first, SHALL then ask the chat's terminal with OSC 11 and wait at most 100 ms, and SHALL treat no answer as dark.

#### Scenario: Light terminal
- **WHEN** no theme is saved and the terminal reports a white background
- **THEN** the session starts in `default-light`

#### Scenario: Silent terminal
- **WHEN** no theme is saved and the terminal does not answer within 100 ms
- **THEN** the session starts in `default` without delay beyond that limit

#### Scenario: Saved choice wins
- **WHEN** a theme is saved
- **THEN** no query is sent and the saved theme is used

### Requirement: The theme picker previews

The system SHALL repaint the screen in the highlighted theme while the theme picker is open without saving it, SHALL apply and save the highlighted theme on Enter, and SHALL restore the theme active when the picker opened on Esc, Ctrl+C, or any other close.

#### Scenario: Move
- **WHEN** the user opens `/theme` and presses Down
- **THEN** the screen repaints in the next theme and the saved theme is unchanged

#### Scenario: Cancel
- **WHEN** the user then presses Esc
- **THEN** the screen returns to the theme active before the picker opened

#### Scenario: Choose
- **WHEN** the user presses Enter on `nord`
- **THEN** `nord` stays active after the picker closes and is saved

### Requirement: A root change is reported once

The system SHALL report a successful session root change with one line that names the new root, with the home directory as `~`, and the prompt cache reset, and SHALL NOT repeat it.

#### Scenario: Choose a folder
- **WHEN** the user chooses `~/Projects/imp` in the `/cd` browser
- **THEN** the transcript gains exactly one line, `cwd: ~/Projects/imp (prompt cache reset)`

#### Scenario: Typed path
- **WHEN** the user runs `/cd ~/Projects/imp`
- **THEN** the same single line appears

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

### Requirement: Mouse capture can be turned off

The system SHALL read a `mouse=on|off` setting, defaulting to `on`, and SHALL send no mouse-reporting request when it is `off`.

#### Scenario: Off
- **WHEN** the config sets `mouse=off`
- **THEN** the chat sends no mouse-reporting request and the wheel behaves as the terminal decides

### Requirement: Tables are drawn to fit

The system SHALL draw a markdown table with box lines in the border colour and bold header cells, SHALL align columns as the separator row says, SHALL render inline markdown inside cells, SHALL shrink the widest columns and wrap their cells when the table is wider than the screen, and SHALL show each row as `header: value` lines when the columns cannot fit.

#### Scenario: Fits
- **WHEN** a three-column table fits the screen
- **THEN** it has a top line, a line under the header, and a bottom line, and every row is one line

#### Scenario: Too wide
- **WHEN** a table's natural width exceeds the screen
- **THEN** its widest column wraps inside its cell and no row is wider than the screen

#### Scenario: Far too wide
- **WHEN** even three-character columns cannot fit
- **THEN** each row becomes lines of `header: value`

#### Scenario: Piped
- **WHEN** line mode prints a reply with a table
- **THEN** the table stays as markdown text

### Requirement: Shown text cannot drive the terminal

The system SHALL remove control characters and escape sequences, other than newlines and tabs, from model and tool text before showing it in either front-end, SHALL show tabs as spaces in the chat, and SHALL keep its own escape sequences for colour, links, and layout.

#### Scenario: Clipboard escape
- **WHEN** a reply contains an OSC 52 clipboard sequence
- **THEN** nothing reaches the terminal but the surrounding text

#### Scenario: Colour codes in tool output
- **WHEN** a command prints ANSI colour codes
- **THEN** its preview shows the plain text without the codes

### Requirement: A keys card lists the chat keys

The system SHALL open a keys card listing the chat keys in the picker overlay when the user types `?` into an empty composer, SHALL close it with Esc or `?`, and SHALL show the keys that act in every overlay and confirmation in that view's bottom edge or hint.

#### Scenario: Keys card
- **WHEN** the composer is empty and the user types `?`
- **THEN** the keys card opens and nothing is sent

#### Scenario: Question mark in text
- **WHEN** the composer holds `why` and the user types `?`
- **THEN** the composer reads `why?` and no card opens

### Requirement: Redraws reuse rendered rows

The system SHALL keep each transcript entry's rendered rows between frames, SHALL render an entry again only when its text, its role, the terminal width, the markdown setting, or the blank row before it changes, and SHALL draw the same screen as a full render.

#### Scenario: Streaming
- **WHEN** a reply streams at the end of a long transcript
- **THEN** only the streaming entry is rendered again on each frame

#### Scenario: Resize
- **WHEN** the terminal width changes
- **THEN** every entry is rendered again at the new width

#### Scenario: Same screen
- **WHEN** a frame is drawn from cached rows
- **THEN** it matches a frame drawn with an empty cache

### Requirement: The idle loop sleeps

The system SHALL wait at most 50 ms between loop passes while a turn streams, a spinner turns, or within two seconds of a key or event, at most one second while a prompt waits, and at most five seconds otherwise, and SHALL wake at once on a key or a terminal resize.

#### Scenario: Idle
- **WHEN** nothing streams, nothing waits, and no key has arrived for two seconds
- **THEN** the loop waits up to five seconds

#### Scenario: Streaming
- **WHEN** a turn streams in any live session
- **THEN** the loop waits at most 50 ms

#### Scenario: Prompt
- **WHEN** an approval waits in view
- **THEN** the loop waits at most one second, so the prompt still expires on time

#### Scenario: Resize
- **WHEN** the terminal is resized while idle
- **THEN** the screen redraws at once

### Requirement: Shown output keeps its colour

The system SHALL keep colour and text-style sequences in output shown to the user, SHALL drop every other escape sequence and control byte, SHALL cut each row to the screen width by its visible characters with its sequences whole and a reset at the end, SHALL show typed `!` command output this way, and SHALL show plain text when colour is off.

#### Scenario: A picture
- **WHEN** the user runs `!chafa --format symbols photo.jpg`
- **THEN** the picture appears in the transcript in colour

#### Scenario: Escapes that move
- **WHEN** output holds a cursor move, a screen clear, or a title change
- **THEN** none of them reaches the terminal

#### Scenario: Colour off
- **WHEN** the config sets `color=never`
- **THEN** the same output appears without colour

### Requirement: Fenced blocks render through a configured command

The system SHALL render a closed fenced block whose language has a `render.<lang>` command by running that command with the block's text on stdin and `COLUMNS` set to the width, SHALL show its output in place of the block, SHALL show the block as code when the command fails or runs past two seconds, SHALL run each command at most once per width and text, and SHALL keep the block's source in the conversation.

#### Scenario: Mermaid
- **WHEN** the config holds `render.mermaid = mmd` and a reply holds a closed ```` ```mermaid ```` block
- **THEN** the diagram appears in place of the block

#### Scenario: Failure
- **WHEN** the command exits non-zero
- **THEN** the block appears as code

#### Scenario: Streaming
- **WHEN** the block's closing fence has not arrived yet
- **THEN** the block appears as code and the command has not run

#### Scenario: Redraw
- **WHEN** the screen redraws with the same width
- **THEN** the command does not run again

### Requirement: Suggestions are accepted with one key

The system SHALL draw a suggestion greyed out in an empty input with no key hint after it, SHALL fill the input with it on Tab or Right, SHALL drop it as soon as the user types, sends, or a turn starts, and SHALL NOT send it on Enter alone.

#### Scenario: Accept
- **WHEN** the input is empty, `run the tests` is suggested, and the user presses Tab then Enter
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

### Requirement: The active step is always visible

The system SHALL, while a task runs, end the transcript with one activity row: the running tool with its elapsed seconds, or `thinking` with elapsed seconds when no tool runs and nothing has streamed since the last step, and SHALL show a step waiting on the user with the accent bar, `waiting for you`, and no spinner.

#### Scenario: Thinking
- **WHEN** a message was sent two seconds ago and no token has arrived
- **THEN** the last transcript row reads `thinking · 2s`

#### Scenario: Waiting
- **WHEN** a command confirmation is open
- **THEN** its tool row starts with `▌`, ends with `waiting for you`, and shows no spinner frame

### Requirement: Approvals show the whole command

The system SHALL, while a labelled command waits for approval, draw the full command above the approval row, wrapped at the screen width and capped at a third of the screen.

#### Scenario: Narrow screen
- **WHEN** a 300-character command labelled `Run test suite` asks at 65 columns
- **THEN** every part of the command up to the cap is on screen and no row is wider than 65 columns

### Requirement: A task reports its time, steps, tokens, and cost

The system SHALL count usage from every request, SHALL show the elapsed time, steps, and output tokens on the right of the status line while a task runs, and SHALL, at verbosity medium or high, follow each finished task with one dim line giving time, steps, input tokens with their cached share, output tokens, and cost when known, which is never sent to the model.

#### Scenario: Multi-step cost
- **WHEN** a turn makes three requests
- **THEN** the cost counts all three

#### Scenario: Finish line
- **WHEN** a task of five steps ends at verbosity medium
- **THEN** a dim line such as `42s · 5 steps · 18K in (92% cached) · 1.4K out · $0.004` follows the reply

#### Scenario: Low
- **WHEN** verbosity is low
- **THEN** no finish line is drawn

#### Scenario: Line mode
- **WHEN** a task ends in line mode
- **THEN** the same line is printed to stderr

### Requirement: Goals are set with /goal

The system SHALL set a goal and start its first round on `/goal <text>`, SHALL show the goal and its round on `/goal`, SHALL clear it on `/goal off` or Esc or a sent message, and SHALL show `goal n/N` in the status line while one runs.

#### Scenario: Start
- **WHEN** the user types `/goal all tests pass`
- **THEN** a turn starts on it and the status line shows `goal 1/5`

#### Scenario: Esc
- **WHEN** the user presses Esc during a goal round
- **THEN** the round is cancelled and no further round starts

### Requirement: Tool rows name paths from the session directory

The system SHALL show a path under the session directory in a tool row relative to that directory, and a path under the scratch folder as `$IMP_TMP`, both live and when a session is replayed.

#### Scenario: A read inside the project

- **WHEN** the agent reads `/home/m/shop/src/cart.py` in a session rooted at `/home/m/shop`
- **THEN** the tool row reads `read src/cart.py`

#### Scenario: A download into scratch

- **WHEN** a labelled call writes into the scratch folder
- **THEN** the tool row names it under `$IMP_TMP`

### Requirement: Help fits narrow screens

The system SHALL put `(default: X)` on its own row, SHALL indent long-only flags past the short-flag column, SHALL put each example's description above its command, SHALL move descriptions under their cells when fewer than 24 columns remain, and SHALL colour `--help` only on a terminal without `NO_COLOR`.

#### Scenario: Long-only flag indent
- **WHEN** `imp --help` lists `-c, --continue` and `--rename`
- **THEN** `--rename` starts in the same column as `--continue`

#### Scenario: Descriptions under cells
- **WHEN** the width is 40 columns
- **THEN** `-l, --list` is on its own row and `list sessions and exit` is on the next, indented six

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

### Requirement: The draft edits by word and by row

The system SHALL move the caret by word on Option+←/→ and Alt+B/F, SHALL delete the word before the caret on Option+Backspace and Ctrl+W and the word after it on Option+Delete and Alt+D, SHALL extend the selection by word on Shift+Option+←/→, SHALL go to the start of the caret's row on Ctrl+A, SHALL delete to the start of the row on Ctrl+U and to its end on Ctrl+K, and SHALL list these keys on the keys card.

#### Scenario: Delete a word
- **WHEN** the draft is `run the tests` with the caret at the end and the user presses Option+Backspace
- **THEN** the draft is `run the `

#### Scenario: Replace a word
- **WHEN** the caret is after `npm` and the user presses Shift+Option+← and types `pnpm`
- **THEN** `npm` is replaced by `pnpm`

#### Scenario: Line keys
- **WHEN** the draft's second row is `second` with the caret after `sec` and the user presses Ctrl+K
- **THEN** that row is `sec` and the first row is unchanged

#### Scenario: Esc alone
- **WHEN** the user presses Esc and nothing follows
- **THEN** it acts as Esc, not as Alt
