## ADDED Requirements

### Requirement: The active step is always visible

The system SHALL, while a task runs, end the transcript with one activity row: the running tool with its elapsed seconds, or `thinking` with elapsed seconds when no tool runs and nothing has streamed since the last step, and SHALL show a step waiting on the user with the accent bar, `waiting for you`, and no spinner.

#### Scenario: Thinking
- **WHEN** a message was sent two seconds ago and no token has arrived
- **THEN** the last transcript row reads `thinking · 2s`

#### Scenario: Waiting
- **WHEN** a command confirmation is open
- **THEN** its tool row starts with `▌`, ends with `waiting for you`, and shows no spinner frame

### Requirement: Approvals show the whole command

The system SHALL, while a labelled command waits for approval, draw the full command above the approval row, wrapped at the screen width and capped at a third of the screen.

#### Scenario: Narrow screen
- **WHEN** a 300-character command labelled `Run test suite` asks at 65 columns
- **THEN** every part of the command up to the cap is on screen and no row is wider than 65 columns

### Requirement: A task reports its time, steps, tokens, and cost

The system SHALL count usage from every request, SHALL show the elapsed time, steps, and output tokens on the right of the status line while a task runs, and SHALL, at verbosity medium or high, follow each finished task with one dim line giving time, steps, input tokens with their cached share, output tokens, and cost when known, which is never sent to the model.

#### Scenario: Multi-step cost
- **WHEN** a turn makes three requests
- **THEN** the cost counts all three

#### Scenario: Finish line
- **WHEN** a task of five steps ends at verbosity medium
- **THEN** a dim line such as `42s · 5 steps · 18K in (92% cached) · 1.4K out · $0.004` follows the reply

#### Scenario: Low
- **WHEN** verbosity is low
- **THEN** no finish line is drawn

#### Scenario: Line mode
- **WHEN** a task ends in line mode
- **THEN** the same line is printed to stderr
