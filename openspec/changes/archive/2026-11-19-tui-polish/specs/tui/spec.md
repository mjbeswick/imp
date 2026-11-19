# Spec Delta

## MODIFIED Requirements

### Requirement: The interface is one chat column

The system SHALL render a scrolling transcript, an input area of one or more rows, and one dim status line at the bottom, SHALL separate the transcript from the input area with one dim horizontal rule when the terminal has at least 8 rows, SHALL leave one blank row before every user entry except the first, SHALL show one dim welcome row naming the app and the `/` hint while the transcript is empty, leaving the directory to the status line, SHALL NOT use panes, tabs, or widgets, SHALL allow exactly one centred bordered overlay for the picker and no other border, and SHALL tell the model the terminal width before a turn whenever that width has changed since the previous turn.

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

## ADDED Requirements

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
