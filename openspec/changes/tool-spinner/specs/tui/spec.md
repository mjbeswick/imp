# Spec Delta

## MODIFIED Requirements

### Requirement: Tool activity is one line each

The system SHALL render each tool call as one row with what it is doing, an argument summary, and the elapsed time, SHALL animate the leading spinner in that row at least every 100 ms while the call runs, SHALL replace the spinner with a bold `✓` or `✗` in the row's colour when the call finishes, SHALL put a failure preview on its own indented line and wrap continuations to that indent, SHALL update the row in place when the call finishes, SHALL charge only the tool's own work and not a confirmation wait, SHALL show a failure with its reason in the error colour, and SHALL expand the most recent call on request.

#### Scenario: Failed tool
- **WHEN** a tool returns a non-zero exit code
- **THEN** its row is shown in the error colour and still carries the duration

#### Scenario: Bold mark
- **WHEN** a tool row is drawn
- **THEN** the leading `✓` or `✗` is bold and the rest of the row keeps its role colour

#### Scenario: Failure detail
- **WHEN** a command exits non-zero and its first output line is `Changes: render-on-change`
- **THEN** that text starts on its own indented line and is never appended after the exit code

#### Scenario: Wrapped summary
- **WHEN** a tool summary is wider than the terminal
- **THEN** every continuation line begins at the row's content indent

#### Scenario: While it runs
- **WHEN** a tool call is running
- **THEN** one row shows the call with an animated spinner, the status line shows no spinner, and the transcript has no second row for it

#### Scenario: Spinner advances
- **WHEN** a call runs for 300 ms
- **THEN** its leading glyph has advanced through at least two frames

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

## ADDED Requirements

### Requirement: The status line has no spinner

The system SHALL start the status line with the model name and SHALL NOT draw a
spinner, animation frame, or activity glyph there, whether the turn is idle,
streaming text, or running a tool.

#### Scenario: Idle
- **WHEN** no turn is running
- **THEN** the status line begins with the model name

#### Scenario: Streaming text
- **WHEN** the model is streaming an answer
- **THEN** the status line is unchanged apart from its token and cost figures

#### Scenario: Tool running
- **WHEN** a tool row is animating
- **THEN** the status line shows no matching glyph

#### Scenario: After the turn
- **WHEN** the turn ends
- **THEN** the status line keeps the same leading model name
