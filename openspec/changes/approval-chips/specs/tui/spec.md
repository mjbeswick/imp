## MODIFIED Requirements

### Requirement: Confirmation prompts appear inline

The system SHALL ask before a destructive command or a memory write with a single-line prompt in the input area, SHALL ask with the verb alone rather than repeating the command the tool row already names, SHALL show each answer as a chip, a bracketed key then its label, SHALL accept `y` or Enter as approval and `n` or Esc as refusal, SHALL accept `s` as approval for the session and `a` as approval saved to the config where the permissions allow them, SHALL show the seconds left at the right edge of the row, and SHALL refuse and stop the turn when no answer arrives within 60 seconds.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

#### Scenario: Ask once
- **WHEN** a confirmation opens for a command
- **THEN** the command appears once, in the tool row above, and the prompt reads `run it?   [y] yes  [n] no  [s] session  [a] always` with the seconds left at the right edge

#### Scenario: Long command
- **WHEN** the command in a confirmation is longer than the row
- **THEN** the command is cut in the middle in the tool row and the prompt keeps its chips and the seconds left

#### Scenario: Deadline shown
- **WHEN** a confirmation opens
- **THEN** its row ends with the seconds left and the count falls with each frame

#### Scenario: Unknown verb
- **WHEN** the action does not start with run, write, save, or delete
- **THEN** the prompt reads `approve?   [y] yes  [n] no` with the seconds left

#### Scenario: Chip colours
- **WHEN** a confirmation opens in a colour terminal
- **THEN** each key letter is bold in the accent colour and the labels and countdown are muted

#### Scenario: Attention
- **WHEN** a confirmation opens
- **THEN** the terminal bell rings once and does not ring again for that prompt

#### Scenario: No answer
- **WHEN** 60 seconds pass without an answer
- **THEN** the command does not run, the transcript says the turn stopped for want of an answer, and the turn ends

#### Scenario: Interrupt a prompt
- **WHEN** the user presses Ctrl+C while a confirmation is open
- **THEN** the prompt closes, the command does not run, and the turn stops

#### Scenario: Always is not offered
- **WHEN** a confirmation cannot be approved beyond once
- **THEN** the prompt shows only `[y] yes  [n] no`, and `s` and `a` do nothing
