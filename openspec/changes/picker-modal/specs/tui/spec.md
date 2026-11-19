# Spec Delta

## MODIFIED Requirements

### Requirement: The interface is one chat column

The system SHALL render a scrolling transcript, an input area of one or more rows, and one dim status line at the bottom, SHALL NOT use panes, tabs, or widgets, SHALL allow exactly one centred bordered overlay for the picker and no other border, and SHALL tell the model the terminal width before a turn whenever that width has changed since the previous turn.

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

#### Scenario: One overlay
- **WHEN** the picker is open
- **THEN** it is the only bordered element and the chat column resumes unchanged when it closes

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

The system SHALL keep a per-session verbosity with two levels, `low` and `high`, SHALL default to `low`, SHALL change it with `/verbosity low` or `/verbosity high`, SHALL open a picker listing both levels for `/verbosity` with no level in the chat interface, SHALL show one summary line per tool call at `low`, SHALL show every call's arguments and capped output at `high` in both front-ends, SHALL apply the level to calls that start after the command runs, and SHALL NOT write the level to the config file.

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
