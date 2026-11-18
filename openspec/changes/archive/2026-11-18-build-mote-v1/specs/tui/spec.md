# Spec Delta

## Purpose

Defines the terminal interface: what the user sees while a turn runs, which keys exist, what the status line reports, and how the terminal is left after any exit.

## ADDED Requirements

### Requirement: The interface is keyboard only

The system SHALL provide Enter to send, Alt+Enter for a newline, Esc to cancel, Ctrl+R to retry, Ctrl+O to expand the last tool output, Ctrl+T to expand thinking, Ctrl+H to hide thinking, Ctrl+L to clear the view, and Ctrl+C twice to quit.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Quit guard
- **WHEN** the user presses Ctrl+C once
- **THEN** the status line asks for a second press, and no second press within two seconds returns to normal

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

### Requirement: Thinking is collapsed by default

The system SHALL show reasoning content as a single dim line with its token count, expandable inline on request, and SHALL keep the collapsed state across turns.

#### Scenario: Expanded reasoning
- **WHEN** the user expands a thinking line
- **THEN** the full reasoning appears in place, dimmed, and can be collapsed again

### Requirement: Tool activity is one line each

The system SHALL render every tool call as a single dim line containing the tool name, a short argument summary, and the elapsed time, and SHALL allow expanding the most recent tool output.

#### Scenario: Failed tool
- **WHEN** a tool returns a non-zero exit code
- **THEN** its line is shown in the error color and still carries the duration

### Requirement: Two front-ends share one loop

The system SHALL offer a line mode and a full-screen chat mode, SHALL pick the full-screen mode when both stdin and stdout are terminals, SHALL pick the line mode otherwise, and SHALL allow either to be forced with `--cli` or `--chat`.

#### Scenario: Interactive launch
- **WHEN** the user runs `mote` in a terminal
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

The system SHALL render a scrolling transcript, one input line, and one dim status line at the bottom, and SHALL NOT use panes, borders, tabs, or widgets.

#### Scenario: Quiet layout
- **WHEN** the session is idle
- **THEN** the screen shows the transcript, an empty input line, and one status line

#### Scenario: Long transcript
- **WHEN** the transcript exceeds the screen height
- **THEN** it scrolls within the same column and no second pane appears

### Requirement: The status line reports cost signals

The system SHALL show the model name, estimated context tokens, the share of input tokens served from cache, and the running cost for the session, and SHALL show unknown values as such rather than guessing.

#### Scenario: Cache hit reported
- **WHEN** a response reports prompt cache hit and miss token counts
- **THEN** the status line shows the resulting percentage

#### Scenario: Unknown model
- **WHEN** the configured model has no price entry
- **THEN** the status line shows token counts and no cost

### Requirement: Confirmation prompts appear inline

The system SHALL ask before a destructive command or a memory write with a single-line prompt in the input area, SHALL accept `y` or Enter as approval and `n` or Esc as refusal, and SHALL return the refusal to the model as a tool result.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

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

The system SHALL persist submitted prompts to a history file, SHALL recall them with Up and Down in the input line, SHALL skip consecutive duplicates, and SHALL cap the file at 1,000 entries.

#### Scenario: Recall
- **WHEN** the user presses Up
- **THEN** the previous prompt appears in the input line and can be sent unchanged

#### Scenario: Across sessions
- **WHEN** a new session starts
- **THEN** the previous session's prompts are still recallable

#### Scenario: Duplicate suppression
- **WHEN** the same prompt is sent twice in a row
- **THEN** it appears once in history

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
- **WHEN** the user runs `mote --help` in an 80-column terminal
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

The system SHALL treat a command typed by the user as already approved, SHALL NOT prompt for it, and SHALL still refuse anything on the deny list unless the mode is `yolo`.

#### Scenario: No prompt
- **WHEN** the user types `!npm install`
- **THEN** it runs without a confirmation prompt

#### Scenario: Denied
- **WHEN** the user types `!sudo rm -rf /`
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
