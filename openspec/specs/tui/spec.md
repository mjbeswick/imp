# tui Specification

## Purpose
Defines the terminal interface: what the user sees while a turn runs, which keys exist, what the status line reports, and how the terminal is left after any exit.

## Requirements

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

### Requirement: Every rendered line fits the terminal

The system SHALL clamp every drawn row to the terminal width, SHALL disable the terminal's automatic wrapping while drawing and restore it after, SHALL erase each row before writing it, and SHALL clear everything below the status line.

#### Scenario: Narrow terminal
- **WHEN** the terminal is 40 columns and the status line is longer
- **THEN** the status is truncated with an ellipsis and no row wraps onto the next

#### Scenario: Shrinking window
- **WHEN** the window shrinks between frames
- **THEN** the next frame has no leftover text from the wider one

#### Scenario: No debris
- **WHEN** a menu closes
- **THEN** the rows it used are erased and the chat pane grows back

### Requirement: Line mode frames each turn

The system SHALL print a prompt only when stdin is a terminal, SHALL start model output on a fresh line, and SHALL NOT emit a prompt or escape sequences when piped.

#### Scenario: Interactive line mode
- **WHEN** the user types a message and presses Enter
- **THEN** the answer begins on the next line, never appended to the typed line

#### Scenario: Piped
- **WHEN** input is a pipe
- **THEN** output contains no `> ` prompt and no escape bytes

### Requirement: Output streams as it arrives

The system SHALL render each token within 100 ms of receiving it, keep the newest line visible while streaming, and show a caret at the end of incomplete text.

#### Scenario: Long answer
- **WHEN** an answer longer than the pane arrives over several seconds
- **THEN** text grows continuously and the pane follows the tail

#### Scenario: Scroll detaches
- **WHEN** the user scrolls up during streaming
- **THEN** following stops, a line count of unseen output is shown, and End resumes following

#### Scenario: Half-written markup
- **WHEN** a stream stops in the middle of a link or a fence
- **THEN** the partial line renders as plain text and no broken escape sequence reaches the terminal

### Requirement: Thinking is collapsed by default

The system SHALL show reasoning content as a single dim line with its token count, expandable inline on request, and SHALL keep the collapsed state across turns.

#### Scenario: Expanded reasoning
- **WHEN** the user expands a thinking line
- **THEN** the full reasoning appears in place, dimmed, and can be collapsed again

### Requirement: Tool activity is one line each

The system SHALL render every tool call as a single dim line containing what the tool is doing, a short argument summary, and the elapsed time, SHALL update that line in place when the call finishes, SHALL measure only the tool's own work and not time spent waiting for a confirmation, SHALL show a failure with its reason in the error colour, and SHALL allow expanding the most recent call to its full arguments and output.

#### Scenario: Failed tool
- **WHEN** a tool returns a non-zero exit code
- **THEN** its line is shown in the error colour and still carries the duration

#### Scenario: While it runs
- **WHEN** a tool call is running
- **THEN** one row shows the call with a spinner, and the transcript has no second row for it

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

The system SHALL offer a line mode and a full-screen chat mode, SHALL pick the full-screen mode when both stdin and stdout are terminals, SHALL pick the line mode otherwise, and SHALL allow either to be forced with `--cli` or `--chat`.

#### Scenario: Interactive launch
- **WHEN** the user runs `imp` in a terminal
- **THEN** the full-screen chat opens

#### Scenario: Scripted launch
- **WHEN** stdout is a pipe
- **THEN** the line mode runs, printing plain text with no escape sequences

#### Scenario: Forced
- **WHEN** the user passes `--cli` in a terminal
- **THEN** the line mode runs without taking over the screen

#### Scenario: Same behaviour
- **WHEN** the same prompt is sent in either mode
- **THEN** the same tools run and the same history is recorded

### Requirement: The interface is one chat column

The system SHALL render a scrolling transcript, an input area of one or more rows, and one dim status line at the bottom, SHALL NOT use panes, borders, tabs, or widgets, and SHALL tell the model the terminal width before a turn whenever that width has changed since the previous turn.

#### Scenario: Quiet layout
- **WHEN** the session is idle
- **THEN** the screen shows the transcript, an empty input row, and one status line

#### Scenario: Long transcript
- **WHEN** the transcript exceeds the screen height
- **THEN** it scrolls within the same column and no second pane appears

#### Scenario: Taller composer
- **WHEN** the composer holds three rows
- **THEN** the transcript gives up two rows, the status line stays last, and the layout is otherwise unchanged

#### Scenario: Resized window
- **WHEN** the window width changes between turns
- **THEN** the next turn carries one line naming the new width, and later turns at that width carry none

#### Scenario: Piped output
- **WHEN** input is a pipe with no terminal size
- **THEN** no width line is sent

### Requirement: The status line reports cost signals

The system SHALL show the model name, estimated context tokens, the share of input tokens served from cache, and the running cost for the session, and SHALL show unknown values as such rather than guessing.

#### Scenario: Cache hit reported
- **WHEN** a response reports prompt cache hit and miss token counts
- **THEN** the status line shows the resulting percentage

#### Scenario: Unknown model
- **WHEN** the configured model has no price entry
- **THEN** the status line shows token counts and no cost

### Requirement: Confirmation prompts appear inline

The system SHALL ask before a destructive command or a memory write with a single-line prompt in the input area, SHALL accept `y` or Enter as approval and `n` or Esc as refusal, SHALL show the seconds left and keep the answer hint visible on a long command, and SHALL refuse and stop the turn when no answer arrives within 60 seconds.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

#### Scenario: Deadline shown
- **WHEN** a confirmation opens
- **THEN** its row shows the seconds left and the count falls with each frame

#### Scenario: Long command
- **WHEN** the command in a confirmation is longer than the row
- **THEN** the command is cut in the middle and `y/N` with the seconds left stays visible

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

### Requirement: Eight slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/model`, `/reload`, `/mode`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all six commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Command effect
- **WHEN** the user runs `/compact`
- **THEN** compaction runs immediately and the result is reported in the transcript

#### Scenario: Reload
- **WHEN** the user runs `/reload` after editing `AGENTS.md`
- **THEN** the next request carries the new instructions and the interface says the cache was reset

#### Scenario: Duplicate affordance is absent
- **WHEN** the user types `/clear`
- **THEN** it is not a command; Ctrl+L clears the view

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

### Requirement: Help is grouped, aligned, and shared

The system SHALL render `--help` with titled sections (usage, sessions, running, other, examples, environment, files), aligned columns, defaults in brackets, and descriptions wrapped to the terminal width, and SHALL render `/help` from the same command table in both front-ends.

#### Scenario: Flag help
- **WHEN** the user runs `imp --help` in an 80-column terminal
- **THEN** no line exceeds 80 columns, every flag appears once, and defaults are shown

#### Scenario: Same commands everywhere
- **WHEN** the user runs `/help` in chat or line mode
- **THEN** the same seven commands and the same key list appear

#### Scenario: Unknown command
- **WHEN** the user types `/compac`
- **THEN** the reply suggests `/compact`

### Requirement: Slash commands autocomplete

The system SHALL show a menu of matching commands as soon as the input begins with `/`, SHALL rank by prefix before substring, SHALL show at most eight rows with descriptions, and SHALL accept the highlighted entry with Tab or Enter.

#### Scenario: Narrowing
- **WHEN** the user types `/co`
- **THEN** `/compact` is listed and other commands are filtered out

#### Scenario: Accept
- **WHEN** the user presses Tab
- **THEN** the highlighted command is completed in the input line and the menu closes

#### Scenario: Run immediately
- **WHEN** the user presses Enter on the highlighted command
- **THEN** the command runs and the input clears

#### Scenario: Dismiss
- **WHEN** the user presses Esc
- **THEN** the menu closes and the typed text stays

### Requirement: The interface uses colour with meaning

The system SHALL colour by role rather than by text, SHALL use at least six roles — user prompt, assistant text, thinking, tool line, error, accent — SHALL detect truecolor, 256-colour, and no-colour terminals, and SHALL honour `NO_COLOR` and a `color=auto|always|never` setting.

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

### Requirement: Shell commands run from the input line

The system SHALL run any input beginning with `!` as a local shell command instead of sending it to the model, SHALL show the command and its output in the transcript, and SHALL cap that output at 4 KB with the usual spill rule.

#### Scenario: Run a command
- **WHEN** the user types `!git status` and presses Enter
- **THEN** the command runs in the session directory, its output appears in the transcript, and no request is sent to the model

#### Scenario: Failing command
- **WHEN** the command exits non-zero
- **THEN** the exit code and stderr appear in the error role and the session continues

#### Scenario: Large output
- **WHEN** the command writes 1 MB
- **THEN** the transcript shows a short head and tail plus the spill path

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

The system SHALL render assistant text as a markdown subset: headings, bold, italic, inline code, fenced code, bullet and numbered lists, blockquotes, and horizontal rules, and SHALL NOT render tables or images.

#### Scenario: Emphasis
- **WHEN** the model writes `**done** in 2 s`
- **THEN** the row shows `done` in bold and the rest unchanged

#### Scenario: Fenced code
- **WHEN** the model writes a fenced block
- **THEN** its lines are shown dim, without fence markers, and without wrapping inside the block

#### Scenario: Lists and quotes
- **WHEN** the model writes a bullet list and a quote
- **THEN** bullets render with a marker and the quote renders dim and prefixed

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
