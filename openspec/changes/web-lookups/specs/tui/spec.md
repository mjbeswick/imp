## MODIFIED Requirements

### Requirement: Verbosity controls tool detail

The system SHALL keep a per-session verbosity with three levels, `low`, `medium`, and `high`, SHALL start from the config's `verbosity`, defaulting to `medium`, SHALL change it with `/verbosity low`, `/verbosity medium`, or `/verbosity high`, SHALL show one summary line per step at `low`, add a short result after the summary at `medium`, show every call's arguments and capped output at `high` in both front-ends, SHALL apply the level to later calls, and SHALL NOT write a `/verbosity` change to the config file.

#### Scenario: Default
- **WHEN** a session starts and the config sets no verbosity
- **THEN** tool calls render as one summary line each, followed by a short result such as `→ 200 json · 12 KB`

#### Scenario: High
- **WHEN** the user runs `/verbosity high` and the next tool call runs
- **THEN** that call shows its arguments and capped output in the transcript

#### Scenario: Back to low
- **WHEN** the user runs `/verbosity low`
- **THEN** later calls return to one summary line without a result and earlier rows stay as they were drawn

#### Scenario: Fetch label
- **WHEN** the model fetches a URL with a long query string
- **THEN** the row names the host and path without the query

#### Scenario: Line mode
- **WHEN** the user runs `/verbosity high` in line mode
- **THEN** each following call prints its arguments and capped output as plain text

#### Scenario: No argument
- **WHEN** the user runs `/verbosity` with no level in the chat interface
- **THEN** the picker lists `low`, `medium`, and `high` with the current level highlighted and choosing one applies it

#### Scenario: Line mode no argument
- **WHEN** the user runs `/verbosity` with no level in line mode
- **THEN** the transcript reports the current level and no picker opens

#### Scenario: Unknown level
- **WHEN** the user runs `/verbosity loud`
- **THEN** the transcript refuses the value and names `low`, `medium`, and `high`
