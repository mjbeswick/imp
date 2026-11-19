# Spec Delta

## MODIFIED Requirements

### Requirement: Tool activity is one line each

The system SHALL render each tool call as one row with what it is doing, an argument summary, and the elapsed time, SHALL bold the leading `✓` or `✗` in the row's colour, SHALL put a failure preview on its own indented line and wrap continuations to that indent, SHALL update the row in place when the call finishes, SHALL charge only the tool's own work and not a confirmation wait, SHALL show a failure with its reason in the error colour, and SHALL expand the most recent call on request.

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

## REMOVED Requirements

### Requirement: Nine slash commands, no more

**Reason**: `/verbosity` is a tenth command that controls how much detail each tool row carries.

**Migration**: the ten-command requirement below replaces it; `/clear`, `/model`, `/reload` as a mode, `/mode`, and `/cost` remain absent.

## ADDED Requirements

### Requirement: Ten slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/yolo`, `/verbosity`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/model`, `/reload` as a mode, `/mode`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all ten commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Command effect
- **WHEN** the user runs `/compact`
- **THEN** compaction runs immediately and the result is reported in the transcript

#### Scenario: Reload
- **WHEN** the user runs `/reload` after editing `AGENTS.md`
- **THEN** the next request carries the new instructions and the interface says the cache was reset

#### Scenario: Yolo on
- **WHEN** the user runs `/yolo` in a session launched in `normal`
- **THEN** the mode becomes `yolo`, the path jail turns off, the transcript names both, and the config file is untouched

#### Scenario: Yolo off
- **WHEN** the user runs `/yolo` again in that session
- **THEN** the mode returns to `normal` and the path jail returns

#### Scenario: Duplicate affordance is absent
- **WHEN** the user types `/clear`
- **THEN** it is not a command; Ctrl+L clears the view

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

### Requirement: Verbosity controls tool detail

The system SHALL keep a per-session verbosity with two levels, `low` and `high`, SHALL default to `low`, SHALL change it with `/verbosity low` or `/verbosity high`, SHALL show one summary line per tool call at `low`, SHALL show every call's arguments and capped output at `high` in both front-ends, SHALL apply the level to calls that start after the command runs, and SHALL NOT write the level to the config file.

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
- **WHEN** the user runs `/verbosity` with no level
- **THEN** the transcript reports the current level and the command does nothing else

#### Scenario: Unknown level
- **WHEN** the user runs `/verbosity loud`
- **THEN** the transcript refuses the value and names `low` and `high`
