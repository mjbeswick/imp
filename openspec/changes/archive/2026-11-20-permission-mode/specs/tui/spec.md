# Spec Delta

## MODIFIED Requirements

### Requirement: Twenty slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/mode`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/stash`, `/rename`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Rename
- **WHEN** the user runs `/rename fix the parser`
- **THEN** the current session carries that title and no request is sent

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

### Requirement: Yolo applies when the command runs

The system SHALL apply `/mode <name>` to the shared mode before its worker command is handled, SHALL honour the new mode for the rest of the running turn, SHALL keep it for the session only without saving it, SHALL name the new mode in the transcript, SHALL report an unknown name as an error listing `normal`, `strict`, and `yolo`, and SHALL keep the deny list and sudo gate in force.

#### Scenario: Enabled mid-turn
- **WHEN** a confirmation would appear and the user has just run `/mode yolo`
- **THEN** the call runs without a prompt

#### Scenario: Disabled
- **WHEN** the user runs `/mode normal`
- **THEN** later calls ask as before

#### Scenario: No double toggle
- **WHEN** `/mode` commands are queued while a turn runs
- **THEN** each carries its target mode and the last one wins

#### Scenario: Hard stops
- **WHEN** the mode is yolo and a command is on the deny list
- **THEN** it is still refused

#### Scenario: Strict mid-session
- **WHEN** the user runs `/mode strict`
- **THEN** every later command asks, and the config file is unchanged

#### Scenario: Unknown mode
- **WHEN** the user runs `/mode wild`
- **THEN** an error lists `normal`, `strict`, and `yolo` and the mode is unchanged

### Requirement: Yolo shows in colour

The system SHALL paint the composer prompt glyph and the rule above the composer in the error colour while the live mode is `yolo`, SHALL paint the line-mode prompt the same way, SHALL leave the status line text unchanged, and SHALL emit nothing extra when colour is off.

#### Scenario: Yolo on
- **WHEN** the user runs `/mode yolo` in a colour terminal
- **THEN** the next frame paints `> ` and the rule in the error colour

#### Scenario: Back to normal
- **WHEN** the user runs `/mode normal`
- **THEN** the glyph and rule return to their usual colours

#### Scenario: No colour
- **WHEN** colour is off and the mode is `yolo`
- **THEN** the frame is identical to the `normal` frame
