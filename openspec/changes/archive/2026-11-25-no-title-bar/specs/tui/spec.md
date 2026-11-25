# Spec Delta

## MODIFIED Requirements

### Requirement: The interface is one chat column

The system SHALL render a scrolling transcript, an input area of one or more rows, and one dim status line at the bottom, SHALL draw a rule above the input only when it carries a label or a warning tint and SHALL draw none otherwise, SHALL NOT use panes, tabs, or widgets, SHALL allow exactly one centred bordered overlay for the picker and no other border, and SHALL tell the model the terminal width, and the height when known, before a turn whenever either has changed since the previous turn.

#### Scenario: Quiet layout
- **WHEN** the session is idle and the transcript is empty
- **THEN** the screen shows the welcome row, an empty input row, and one status line

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
- **THEN** the transcript gives up two rows, the rule when drawn and the status line keep their places, and the layout is otherwise unchanged

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

## REMOVED Requirements

### Requirement: A title bar names the session

**Reason**: the row is better spent on the transcript. The terminal title names the session and the status line carries the position.

**Migration**: replaced by "The terminal title names the session".

## ADDED Requirements

### Requirement: The terminal title names the session

The system SHALL set the terminal title to `imp` before the session's first message and to `<title> · imp` after it, SHALL update it on the first frame and whenever the session in view, its title, or a rename changes it, SHALL strip control characters, SHALL cap it at 80 characters, SHALL NOT reserve a screen row for a title bar, and SHALL NOT draw a clock.

#### Scenario: Before the first message
- **WHEN** imp starts with no message sent
- **THEN** the terminal title is `imp`

#### Scenario: After the first message
- **WHEN** the session is titled `fix the parser`
- **THEN** the terminal title is `fix the parser · imp`

#### Scenario: Rename
- **WHEN** the user renames the session to `fix sync`
- **THEN** the terminal title becomes `fix sync · imp`

#### Scenario: Session switch
- **WHEN** the user switches to another live session
- **THEN** the terminal title becomes that session's title

#### Scenario: Control characters
- **WHEN** the first message contains an escape sequence
- **THEN** the terminal title carries none of it

#### Scenario: No title row
- **WHEN** the screen has 24 rows
- **THEN** no row carries a title bar or a clock
