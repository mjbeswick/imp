# Spec Delta

## MODIFIED Requirements

### Requirement: Confirmation prompts appear inline

The system SHALL ask before a destructive command or a memory write with a single-line prompt in the input area, SHALL accept `y` or Enter as approval and `n` or Esc as refusal, SHALL show the seconds left before it gives up, SHALL keep the answer hint and the deadline visible when the command is longer than the row, SHALL ring the terminal bell once when the prompt opens, SHALL refuse and stop the turn when no answer arrives within 60 seconds, and SHALL return the refusal to the model as a tool result.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

#### Scenario: Deadline shown
- **WHEN** a confirmation opens
- **THEN** its row shows the seconds left and the count falls with each frame

#### Scenario: Long command
- **WHEN** the command in a confirmation is longer than the row
- **THEN** the command is cut in the middle and `y/N` with the seconds left stays visible

#### Scenario: Attention
- **WHEN** a confirmation opens
- **THEN** the terminal bell rings once and does not ring again for that prompt

#### Scenario: No answer
- **WHEN** 60 seconds pass without an answer
- **THEN** the command does not run, the transcript says the turn stopped for want of an answer, and the turn ends

#### Scenario: Interrupt a prompt
- **WHEN** the user presses Ctrl+C while a confirmation is open
- **THEN** the prompt closes, the command does not run, and the turn stops

### Requirement: Typed commands bypass confirmation but not denial

The system SHALL treat a command typed by the user as already approved, SHALL NOT prompt for it, and SHALL still refuse anything on the deny list in every mode.

#### Scenario: No prompt
- **WHEN** the user types `!npm install`
- **THEN** it runs without a confirmation prompt

#### Scenario: Denied
- **WHEN** the user types `!sudo rm -rf /`
- **THEN** it is refused with a message naming the rule

#### Scenario: Denied in yolo
- **WHEN** the mode is `yolo` and the user types `!rm -rf /`
- **THEN** it is refused with a message naming the rule
