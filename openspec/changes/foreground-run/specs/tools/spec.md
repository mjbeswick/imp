## ADDED Requirements

### Requirement: A command can run in the foreground

The system SHALL, for a `bash` call with `foreground: true` in the chat, ask before running it in `normal` and `strict` regardless of allows, SHALL then hand the terminal to the command with no timeout, SHALL restore the interface when it exits, SHALL return only its exit code, and SHALL refuse the call in line mode.

#### Scenario: Run an app
- **WHEN** the model calls `bash` with `htop` and `foreground: true`, and the user answers yes
- **THEN** htop has the screen and keyboard, and when it exits the chat redraws and the result reads `exit 0 (ran in the foreground; output not captured)`

#### Scenario: Allowed shape still asks
- **WHEN** `npm run dev` is allowed always and the model runs it with `foreground: true` in `normal`
- **THEN** imp asks before running it

#### Scenario: Yolo
- **WHEN** the mode is `yolo`
- **THEN** the command runs in the foreground without asking

#### Scenario: Refused
- **WHEN** the user answers no
- **THEN** nothing runs and the result says it was refused

#### Scenario: Line mode
- **WHEN** a foreground call arrives in line mode
- **THEN** it is refused with `foreground needs the chat interface`
