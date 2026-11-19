# Spec Delta

## MODIFIED Requirements

### Requirement: Tools respect the permission decision

The system SHALL classify every tool action before executing it against the live permission mode, SHALL refuse actions the policy denies, SHALL wait for the user on actions the policy asks about, and SHALL return the refusal to the model as a result rather than an error.

#### Scenario: Denied by policy
- **WHEN** a command matches the deny list
- **THEN** it is not run and the model is told it was refused

#### Scenario: Asked and approved
- **WHEN** an action requires confirmation and the user approves
- **THEN** it runs and the result is returned normally

#### Scenario: Asked and refused
- **WHEN** the user refuses
- **THEN** nothing happens and the model receives a result saying the user declined

#### Scenario: Mode changes mid-turn
- **WHEN** the user enables yolo after the turn started
- **THEN** the turn's next tool call does not ask for confirmation

