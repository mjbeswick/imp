# Spec Delta

## MODIFIED Requirements

### Requirement: The interface uses colour with meaning

The system SHALL colour by role rather than by text, SHALL use at least six roles — user prompt, assistant text, thinking, tool line, error, accent — SHALL paint inline code bold in the tool colour and keep fenced blocks and quotes dim, SHALL detect truecolor, 256-colour, and no-colour terminals, and SHALL honour `NO_COLOR` and a `color=auto|always|never` setting.

#### Scenario: Truecolor terminal
- **WHEN** `COLORTERM` advertises truecolor
- **THEN** roles render as 24-bit RGB

#### Scenario: Plain terminal
- **WHEN** the terminal supports 256 colours only
- **THEN** the same roles render from a 256-colour palette that keeps the same contrast relationships

#### Scenario: No colour
- **WHEN** `NO_COLOR` is set or the setting is `never`
- **THEN** no escape sequences are emitted and the layout is unchanged

#### Scenario: Error stands out
- **WHEN** a tool exits non-zero or a request fails
- **THEN** that line renders in the error role and remains readable without colour

#### Scenario: Inline code is not dim
- **WHEN** a reply contains `` `bash` `` inside a sentence
- **THEN** that span is bold in the tool colour, not the thinking grey

#### Scenario: Fenced code stays dim
- **WHEN** a reply contains a fenced block
- **THEN** its lines are dim and not bold

### Requirement: Shell commands run from the input line

The system SHALL run any input beginning with `!` as a local shell command instead of sending it to the model, SHALL show the command and its output in the transcript, SHALL show `$ ` as the prompt and a shell status while such a draft is open, and SHALL cap that output at 4 KB with the usual spill rule.

#### Scenario: Run a command
- **WHEN** the user types `!git status` and presses Enter
- **THEN** the command runs in the session directory, its output appears in the transcript, and no request is sent to the model

#### Scenario: Failing command
- **WHEN** the command exits non-zero
- **THEN** the exit code and stderr appear in the error role and the session continues

#### Scenario: Large output
- **WHEN** the command writes 1 MB
- **THEN** the transcript shows a short head and tail plus the spill path

#### Scenario: Shell draft
- **WHEN** the composer holds `!git status`
- **THEN** the prompt reads `$ ` and the status row says the command is local

#### Scenario: Back to the model
- **WHEN** the draft no longer starts with `!`
- **THEN** the prompt returns to `> ` and the status row reports model and cost
