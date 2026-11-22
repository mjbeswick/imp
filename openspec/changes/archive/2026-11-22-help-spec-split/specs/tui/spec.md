## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Help fits narrow screens

The system SHALL put `(default: X)` on its own row, SHALL indent long-only flags past the short-flag column, SHALL put each example's description above its command, SHALL move descriptions under their cells when fewer than 24 columns remain, and SHALL colour `--help` only on a terminal without `NO_COLOR`.

#### Scenario: Long-only flag indent
- **WHEN** `imp --help` lists `-c, --continue` and `--rename`
- **THEN** `--rename` starts in the same column as `--continue`

#### Scenario: Descriptions under cells
- **WHEN** the width is 40 columns
- **THEN** `-l, --list` is on its own row and `list sessions and exit` is on the next, indented six
