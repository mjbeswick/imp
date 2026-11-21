## ADDED Requirements

### Requirement: Agent commands see the screen size

The system SHALL set `COLUMNS` and `LINES` to the current terminal size in the environment of commands the agent runs, when that size is known, and SHALL leave commands the user types unchanged.

#### Scenario: Picture sized to the screen
- **WHEN** the agent runs `echo "$COLUMNS $LINES"` in an 80 by 24 terminal
- **THEN** the output is `80 24`

#### Scenario: No terminal
- **WHEN** imp runs in line mode with no terminal size
- **THEN** the command sees the variables it inherited
