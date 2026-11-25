# Spec Delta

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
- **THEN** the first row is the transcript and no row shows a clock
