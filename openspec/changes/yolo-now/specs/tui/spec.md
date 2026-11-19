# Spec Delta

## ADDED Requirements

### Requirement: Yolo applies when the command runs

The system SHALL apply `/yolo` to the shared mode before its worker command is handled, SHALL stop confirmations for the rest of the running turn, SHALL switch back to the launch mode on the next `/yolo`, SHALL label the session and launch modes in the transcript, and SHALL keep the deny list and sudo gate in force.

#### Scenario: Enabled mid-turn
- **WHEN** a confirmation would appear and the user has just run `/yolo`
- **THEN** the call runs without a prompt

#### Scenario: Disabled
- **WHEN** the user runs `/yolo` again
- **THEN** the launch mode returns and later calls ask as before

#### Scenario: No double toggle
- **WHEN** `/yolo` commands are queued while a turn runs
- **THEN** each carries its target mode and the last one wins

#### Scenario: Hard stops
- **WHEN** the mode is yolo and a command is on the deny list
- **THEN** it is still refused
