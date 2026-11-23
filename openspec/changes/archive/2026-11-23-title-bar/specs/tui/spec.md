## ADDED Requirements

### Requirement: A title bar names the session

The system SHALL draw a title bar as the top row when the screen has at least eight rows, on the theme's select background or reversed without colour, with the session title on the left, `imp` before the first message, preceded by the position while several sessions are live, and the local time as `HH:MM` on the right, SHALL shorten the title before the time, and SHALL repaint it when the minute changes.

#### Scenario: Title and time
- **WHEN** the session is titled `fix the parser` and it is 14:05
- **THEN** the top row starts with ` fix the parser` and ends with `14:05 `

#### Scenario: Narrow
- **WHEN** the title is longer than the row
- **THEN** the title is cut with `…` and the time stays whole

#### Scenario: Short screen
- **WHEN** the screen has seven rows
- **THEN** no title bar is drawn
