# Spec Delta

## MODIFIED Requirements

### Requirement: Confirmation prompts appear inline

The system SHALL ask before a destructive command or a memory write with a single-line prompt in the input area, SHALL accept `y` or Enter as approval and `n` or Esc as refusal, SHALL show the seconds left before it gives up, SHALL ring the terminal bell once when the prompt opens, SHALL refuse and stop the turn when no answer arrives within 60 seconds, and SHALL return the refusal to the model as a tool result.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

#### Scenario: Deadline shown
- **WHEN** a confirmation opens
- **THEN** its row shows the seconds left and the count falls with each frame

#### Scenario: Attention
- **WHEN** a confirmation opens
- **THEN** the terminal bell rings once and does not ring again for that prompt

#### Scenario: No answer
- **WHEN** 60 seconds pass without an answer
- **THEN** the command does not run, the transcript says the turn stopped for want of an answer, and the turn ends

#### Scenario: Interrupt a prompt
- **WHEN** the user presses Ctrl+C while a confirmation is open
- **THEN** the prompt closes, the command does not run, and the turn stops
