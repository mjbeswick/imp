## MODIFIED Requirements

### Requirement: A title bar names the session

The system SHALL draw a title bar as the top row when the screen has at least eight rows, on the theme's select background, or with `title_gradient` on and truecolor a gradient from it toward the accent, reversed without colour, with the session title on the left, `imp` before the first message, preceded by the position while several sessions are live, and the local time as `HH:MM` on the right, SHALL shorten the title before the time, and SHALL repaint it when the minute changes.

#### Scenario: Title and time
- **WHEN** the session is titled `fix the parser` and it is 14:05
- **THEN** the top row starts with ` fix the parser` and ends with `14:05 `

#### Scenario: Narrow
- **WHEN** the title is longer than the row
- **THEN** the title is cut with `…` and the time stays whole

#### Scenario: Short screen
- **WHEN** the screen has seven rows
- **THEN** no title bar is drawn

#### Scenario: Gradient
- **WHEN** `title_gradient` is on and the terminal has truecolor
- **THEN** each cell of the title bar has its own background, shading from the select colour to the accent

#### Scenario: Gradient without truecolor
- **WHEN** `title_gradient` is on and the terminal has 256 colours
- **THEN** the title bar is drawn flat on the select background

### Requirement: Twenty-five slash commands, no more

The system SHALL provide exactly `/help`, `/keys`, `/diff`, `/new`, `/goal`, `/copy`, `/sessions`, `/agent`, `/reload`, `/compact`, `/retry`, `/think`, `/effort`, `/settings`, `/mode`, `/next`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/queue`, `/rename`, and `/quit`, SHALL run them without sending a request to the model except `/retry`, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty-eight commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Rename
- **WHEN** the user runs `/rename fix the parser`
- **THEN** the current session carries that title and no request is sent

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

#### Scenario: Retry
- **WHEN** the user runs `/retry` after a finished reply
- **THEN** the last user message is sent again and the old reply leaves the transcript and the history

#### Scenario: Nothing to retry
- **WHEN** no message was sent, or a turn is running, and the user runs `/retry`
- **THEN** a line says `nothing to retry` or `a turn is running` and nothing is sent

#### Scenario: Effort
- **WHEN** the user runs `/effort high`
- **THEN** the next request asks the provider for high effort and the transcript says `effort: high`

## ADDED Requirements

### Requirement: Settings are a form

The system SHALL open a settings form on `/settings` listing the start mode, thinking, effort, verbosity, mouse, suggestions, diffs, moving to the next waiting session, the title gradient, and the spinner, each with its current value, SHALL step the highlighted value with Right, Enter, or Space and back with Left, SHALL apply it at once, SHALL save it to the config file keeping other lines and comments, and SHALL close on Esc.

#### Scenario: Toggle
- **WHEN** the user opens `/settings`, highlights thinking, and presses Enter
- **THEN** thinking switches, the row shows the new value, and the config file gains `thinking = on`

#### Scenario: Comments kept
- **WHEN** the config holds comments and a `thinking=false` line and the user changes thinking
- **THEN** that line becomes `thinking = on` and every other line is unchanged

#### Scenario: Close
- **WHEN** the user presses Esc
- **THEN** the form closes and the values set stay
