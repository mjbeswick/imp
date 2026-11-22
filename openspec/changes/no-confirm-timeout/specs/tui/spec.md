## MODIFIED Requirements

### Requirement: Confirmation prompts appear inline

The system SHALL ask before a destructive command or a memory write on one row in the input area, naming the verb alone when the tool row names the command, SHALL show each answer as a chip, a bracketed key then its label, SHALL accept `y` or Enter, `n` or Esc, and `s` and `a` where offered, SHALL treat `y` and Enter as the default answer, and SHALL wait until answered, with no deadline and no countdown.

#### Scenario: Declined command
- **WHEN** the model asks to run `rm -rf build` and the user refuses
- **THEN** the model receives a tool result stating the user declined and the turn continues

#### Scenario: Ask once
- **WHEN** a confirmation opens for a command
- **THEN** the command appears once, in the tool row above, and the prompt reads `run it?   [y] yes  [n] no  [s] session  [a] always`

#### Scenario: Long command
- **WHEN** the command in a confirmation is longer than the row
- **THEN** the command is cut in the middle in the tool row and the prompt keeps its chips

#### Scenario: Deadline shown
- **WHEN** a confirmation opens
- **THEN** its row shows the question and chips and no seconds left

#### Scenario: Unknown verb
- **WHEN** the action does not start with run, write, save, or delete
- **THEN** the prompt reads `approve?   [y] yes  [n] no`

#### Scenario: Chip colours
- **WHEN** a confirmation opens in a colour terminal
- **THEN** each key letter is bold in the accent colour and the labels are muted

#### Scenario: Attention
- **WHEN** a confirmation opens
- **THEN** the terminal bell rings once and does not ring again for that prompt

#### Scenario: No answer
- **WHEN** a confirmation has been open for ten minutes
- **THEN** it is still open, the command has not run, and the turn is still waiting

#### Scenario: Enter approves
- **WHEN** the user presses Enter on a confirmation
- **THEN** the command runs as if `y` were pressed

#### Scenario: Interrupt a prompt
- **WHEN** the user presses Ctrl+C while a confirmation is open
- **THEN** the prompt closes, the command does not run, and the turn stops

#### Scenario: Always is not offered
- **WHEN** a confirmation cannot be approved beyond once
- **THEN** the prompt shows only `[y] yes  [n] no`, and `s` and `a` do nothing
