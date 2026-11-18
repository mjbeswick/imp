# Spec Delta

## MODIFIED Requirements

### Requirement: Tool activity is one line each

The system SHALL render every tool call as a single dim line containing what the tool is doing, a short argument summary, and the elapsed time, SHALL update that line in place when the call finishes, SHALL measure only the tool's own work and not time spent waiting for a confirmation, SHALL show a failure with its reason in the error colour, and SHALL allow expanding the most recent call to its full arguments and output.

#### Scenario: Failed tool
- **WHEN** a tool returns a non-zero exit code
- **THEN** its line is shown in the error colour and still carries the duration

#### Scenario: While it runs
- **WHEN** a tool call is running
- **THEN** one row shows the call with a spinner, and the transcript has no second row for it

#### Scenario: When it finishes
- **WHEN** the call returns
- **THEN** the same row carries `✓` or `✗`, the result summary, and the time the tool took

#### Scenario: Confirmation is not charged
- **WHEN** a confirmation is left open for 50 seconds and the command runs for 5 ms
- **THEN** the row reports milliseconds, not the wait

#### Scenario: Refused call
- **WHEN** a write is refused because the path is outside the session
- **THEN** the row shows `✗` with the reason

#### Scenario: Expand the last call
- **WHEN** the user presses Ctrl+O
- **THEN** the most recent call shows its full arguments and output, and pressing it again collapses the row
