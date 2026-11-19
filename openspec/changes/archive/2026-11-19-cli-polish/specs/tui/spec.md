# Spec Delta

## MODIFIED Requirements

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

### Requirement: Fourteen slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/yolo`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/model`, `/reload` as a mode, `/mode`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all fifteen commands appear with a one-line description

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

#### Scenario: Fork
- **WHEN** the user runs `/fork`
- **THEN** the session continues in a new id and the transcript names both ids

#### Scenario: Theme
- **WHEN** the user runs `/theme solarized-dark`
- **THEN** the palette changes and the transcript names the theme

## ADDED Requirements

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

The system SHALL provide four built-in themes named `default`, `mono`, `solarized-dark`, and `gruvbox-dark`, SHALL apply the active theme to both front-ends, SHALL switch with `/theme <name>`, SHALL list the themes and mark the current one for `/theme` with no argument, SHALL reject an unknown name and list the valid ones, SHALL load the saved theme at startup, and SHALL save a switch in the state directory. `mono` SHALL force colour off.

#### Scenario: List
- **WHEN** the user runs `/theme`
- **THEN** the four names appear with the active one marked

#### Scenario: Switch
- **WHEN** the user runs `/theme gruvbox-dark`
- **THEN** the next frame and the next line-mode rows use that palette

#### Scenario: Mono
- **WHEN** the user runs `/theme mono`
- **THEN** no colour is emitted even in a colour terminal

#### Scenario: Unknown
- **WHEN** the user runs `/theme neon`
- **THEN** the transcript refuses it and lists the four names

#### Scenario: Persistence
- **WHEN** the user switches to `solarized-dark` and restarts
- **THEN** the session starts in `solarized-dark`

#### Scenario: Bad saved value
- **WHEN** the state file names a theme that does not exist
- **THEN** the session starts in `default`
