# tui Specification

## Purpose
Defines the terminal interface: what the user sees while a turn runs, which keys exist, what the status line reports, and how the terminal is left after any exit.

## Requirements

### Requirement: The interface is keyboard only

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+R to retry, Ctrl+O to expand the last tool output, Ctrl+T and Ctrl+H to show or hide thinking, Ctrl+L to clear the view, PgUp/PgDn to scroll, and Ctrl+C to clear a non-empty composer or exit when it is empty, SHALL decode those keys whether the terminal sends control bytes or disambiguated `CSI` sequences, and SHALL NOT type escape parameters as text.

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

The system SHALL render a scrolling transcript, an input area of one or more rows, and one dim status line at the bottom, SHALL NOT use panes, tabs, or widgets, SHALL allow exactly one centred bordered overlay for the picker and no other border, and SHALL tell the model the terminal width before a turn whenever that width has changed since the previous turn.

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

The system SHALL ask before a destructive command or a memory write with a single-line prompt in the input area, SHALL ask with the verb alone rather than repeating the command the tool row already names, SHALL accept `y` or Enter as approval and `n` or Esc as refusal, SHALL show the seconds left next to the answer hint, and SHALL refuse and stop the turn when no answer arrives within 60 seconds.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

#### Scenario: Ask once
- **WHEN** a confirmation opens for a command
- **THEN** the command appears once, in the tool row above, and the prompt reads `run it? y/N` with the seconds left

#### Scenario: Long command
- **WHEN** the command in a confirmation is longer than the row
- **THEN** the command is cut in the middle in the tool row and the prompt keeps `y/N` with the seconds left

#### Scenario: Deadline shown
- **WHEN** a confirmation opens
- **THEN** its row shows the seconds left and the count falls with each frame

#### Scenario: Unknown verb
- **WHEN** the action does not start with run, write, save, or delete
- **THEN** the prompt reads `approve? y/N` with the seconds left

#### Scenario: Attention
- **WHEN** a confirmation opens
- **THEN** the terminal bell rings once and does not ring again for that prompt

#### Scenario: No answer
- **WHEN** 60 seconds pass without an answer
- **THEN** the command does not run, the transcript says the turn stopped for want of an answer, and the turn ends

#### Scenario: Interrupt a prompt
- **WHEN** the user presses Ctrl+C while a confirmation is open
- **THEN** the prompt closes, the command does not run, and the turn stops

### Requirement: Questions are answerable by key or by typing

The system SHALL render an `ask` request as a picker above the input line listing the choices with number keys, SHALL offer a custom-answer row reachable by letter key or arrow, and SHALL show the typed answer before sending it.

#### Scenario: Pick by number
- **WHEN** the question offers three choices
- **THEN** pressing 2 selects the second and the answer is sent immediately

#### Scenario: Type an answer
- **WHEN** the user chooses the custom row and types a sentence
- **THEN** that sentence is the answer and it appears in the transcript

#### Scenario: Dismiss
- **WHEN** the user presses Esc on a question
- **THEN** the question closes, the tool result says it was not answered, and the input is ready

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

#### Scenario: Flag help
- **WHEN** the user runs `imp --help` in an 80-column terminal
- **THEN** no line exceeds 80 columns, every flag appears once, and defaults are shown

#### Scenario: Same commands everywhere
- **WHEN** help is printed in the chat, line mode, or with `--help`
- **THEN** the same twenty commands and the same flags are named in each

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

The system SHALL restore cursor visibility, cooked input mode, and the primary screen on normal exit, panic, SIGINT, and SIGTERM.

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

The system SHALL keep a per-session verbosity with two levels, `low` and `high`, SHALL default to `low`, SHALL change it with `/verbosity low` or `/verbosity high`, SHALL show one summary line at `low`, every call's arguments and capped output at `high` in both front-ends, SHALL apply the level to later calls, and SHALL NOT write it to the config file.

#### Scenario: Default
- **WHEN** a session starts and the user has run no verbosity command
- **THEN** tool calls render as one summary line each

#### Scenario: High
- **WHEN** the user runs `/verbosity high` and the next tool call runs
- **THEN** that call shows its arguments and capped output in the transcript

#### Scenario: Back to low
- **WHEN** the user runs `/verbosity low`
- **THEN** later calls return to one summary line and earlier rows stay as they were drawn

#### Scenario: Line mode
- **WHEN** the user runs `/verbosity high` in line mode
- **THEN** each following call prints its arguments and capped output as plain text

#### Scenario: No argument
- **WHEN** the user runs `/verbosity` with no level in the chat interface
- **THEN** the picker lists `low` and `high` with the current level highlighted and choosing one applies it

#### Scenario: Line mode no argument
- **WHEN** the user runs `/verbosity` with no level in line mode
- **THEN** the transcript reports the current level and no picker opens

#### Scenario: Unknown level
- **WHEN** the user runs `/verbosity loud`
- **THEN** the transcript refuses the value and names `low` and `high`

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

### Requirement: Twenty slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/mode`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/stash`, `/rename`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty commands appear with a one-line description

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

The system SHALL suspend the full-screen interface, restore cooked input, and run `$SHELL` for `/shell`, SHALL restore the interface and redraw when it exits, and SHALL run `$SHELL` directly in line mode.

#### Scenario: Chat shell
- **WHEN** the user runs `/shell` in chat
- **THEN** the normal shell prompt appears and returning to it redraws the chat

#### Scenario: Line mode shell
- **WHEN** the user runs `/shell` in line mode
- **THEN** the shell runs and returns to the prompt

#### Scenario: No shell variable
- **WHEN** `$SHELL` is unset
- **THEN** `/bin/sh` is used

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

The system SHALL paint an open confirmation, question, or input prompt in the accent role behind a `▌` prefix, SHALL paint the rule above it in the accent role while it is open, SHALL draw a text prompt's caret the same way as the composer's, and SHALL keep the prompt to one row.

#### Scenario: Confirmation
- **WHEN** a confirmation opens in a colour terminal
- **THEN** its row starts with `▌` in the accent colour and the rule above it is in the accent colour

#### Scenario: No colour
- **WHEN** colour is off and a confirmation opens
- **THEN** the row starts with `▌` and no escape sequence is emitted for colour

#### Scenario: Text prompt caret
- **WHEN** `/rename` asks for a title
- **THEN** the caret is the composer's block caret, not an underscore

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

The system SHALL, when `mouse` is on, ask the terminal for SGR mouse reports while the chat is open, SHALL scroll the transcript three rows per wheel step, SHALL resume following when scrolled to the end, SHALL move a picker's highlight one row per step, SHALL ignore clicks, and SHALL stop mouse reports on exit, on a crash, and while a shell or attached command has the terminal.

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
