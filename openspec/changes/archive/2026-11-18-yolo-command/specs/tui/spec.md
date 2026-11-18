# Spec Delta

## REMOVED Requirements

### Requirement: Eight slash commands, no more

**Reason**: the session needs one switch that changes the permission mode without a restart.

**Migration**: the nine-command requirement below replaces it. `/yolo` is the addition; `/clear`, `/model`, `/reload` as a mode, `/mode`, and `/cost` remain absent.

## ADDED Requirements

### Requirement: Nine slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/yolo`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/model`, `/reload` as a mode, `/mode`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all nine commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Command effect
- **WHEN** the user runs `/compact`
- **THEN** compaction runs immediately and the result is reported in the transcript

#### Scenario: Reload
- **WHEN** the user runs `/reload` after editing `AGENTS.md`
- **THEN** the next request carries the new instructions and the interface says the cache was reset

#### Scenario: Yolo on
- **WHEN** the user runs `/yolo` in a session launched in `normal`
- **THEN** the mode becomes `yolo`, the path jail turns off, the transcript names both, and the config file is untouched

#### Scenario: Yolo off
- **WHEN** the user runs `/yolo` again in that session
- **THEN** the mode returns to `normal` and the path jail returns

#### Scenario: Duplicate affordance is absent
- **WHEN** the user types `/clear`
- **THEN** it is not a command; Ctrl+L clears the view

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block
