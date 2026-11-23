## MODIFIED Requirements

### Requirement: Tab cycles the agent and Shift+Tab the mode

The system SHALL cycle the permission mode through `normal`, `auto`, `yolo`, and `strict` on Shift+Tab, applying it at once, SHALL cycle agent profiles on Tab when the draft is empty and no picker is open, applying a change made during a turn when that turn ends, SHALL write one transcript line for each mode change, and SHALL show the mode in the status line when it is not `normal`.

#### Scenario: Mode
- **WHEN** the mode is `normal` and the user presses Shift+Tab
- **THEN** the mode is `auto` and one line says so

#### Scenario: Yolo
- **WHEN** the mode is `auto` and the user presses Shift+Tab
- **THEN** the mode is `yolo`, the prompt and rule turn to the error colour, and one line says so

#### Scenario: Round trip
- **WHEN** the mode is `strict` and the user presses Shift+Tab
- **THEN** the mode is `normal`

#### Scenario: Agent
- **WHEN** two agents exist, the draft is empty, and the user presses Tab
- **THEN** the other agent is active for the next message

#### Scenario: Mid-turn agent
- **WHEN** a turn is running and the user presses Tab
- **THEN** the prompt names the coming agent and the switch happens when the turn ends

#### Scenario: Text in the draft
- **WHEN** the draft holds `fix` and the user presses Tab
- **THEN** nothing changes

#### Scenario: Agent before the prompt
- **WHEN** more than one agent exists and `plan` is active
- **THEN** the composer's first row starts with `plan>` and no `agent:` line is written on a switch

#### Scenario: One agent
- **WHEN** only one agent exists
- **THEN** the composer starts with `>` alone
