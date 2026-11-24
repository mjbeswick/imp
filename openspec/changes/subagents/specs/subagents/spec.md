## ADDED Requirements

### Requirement: The model delegates a task to a named profile

The system SHALL provide a `task` tool that takes an `agent` name and a `prompt`, SHALL run that profile in a fresh context with the prompt as its only user message, and SHALL return the child's final text as the tool result.

#### Scenario: Delegated review
- **WHEN** the model calls `task` with `agent: "reviewer"` and a prompt
- **THEN** the reviewer profile runs with its instructions and tools, and its final text is the tool result

#### Scenario: Unknown profile
- **WHEN** the model calls `task` with an agent name that does not exist
- **THEN** the tool result refuses and lists the available profile names, and no child runs

#### Scenario: Missing name
- **WHEN** the model calls `task` without `agent`
- **THEN** the tool result refuses and no child runs

### Requirement: A child uses its profile's tools and settings

The system SHALL give a child exactly the tools its profile allows, SHALL use the profile's `model`, `thinking`, and `effort` when it sets them, and SHALL restore the parent's model, thinking, and effort when the child ends.

#### Scenario: Restricted child
- **WHEN** a profile sets `tools: read, bash`
- **THEN** the child's request offers `read` and `bash` and no `write` or `edit`

#### Scenario: Settings restored
- **WHEN** a child with its own model finishes
- **THEN** the next parent request uses the parent's model

### Requirement: Children do not delegate

The system SHALL NOT offer the `task` tool to a child, and SHALL refuse a `task` call made while a child runs.

#### Scenario: Nested delegation
- **WHEN** a child profile lists `task` in its tools
- **THEN** the child's request does not offer `task`

### Requirement: The session permission policy applies to children

The system SHALL route every confirmation and question a child raises to the root interface, SHALL tag the prompt with the child's name, and SHALL NOT let a child relax the session permission policy.

#### Scenario: Child asks before a write
- **WHEN** the session runs in `strict` and a child attempts a write
- **THEN** the confirmation appears, tagged with the child's name, and the child waits for the answer

### Requirement: Child output is shown with its name

The system SHALL show each tool activity of a child in the transcript with the child's name, SHALL NOT show the child's streamed text or reasoning in the transcript, and SHALL show the parent's `task` row as the summary of the delegated work.

#### Scenario: Child tool row
- **WHEN** a child runs `read` on a file
- **THEN** the transcript row names the child and the file, and no child prose appears

### Requirement: Child messages are logged as nested records

The system SHALL write each child message to the parent's session file as a nested record tagged with the parent's `task` call id, SHALL NOT create a session file for the child, and SHALL NOT send nested records to the model.

#### Scenario: Log after delegation
- **WHEN** a turn delegates once and the child makes three tool calls
- **THEN** the session file gains the parent's `task` call and result, plus one nested record per child message, each tagged with the call id, and no new session file exists

#### Scenario: Resume keeps the wire clean
- **WHEN** a session with nested records is resumed
- **THEN** the next request to the model contains only the parent's messages

### Requirement: A child transcript can be inspected

The system SHALL let the user open a child transcript from its `task` row, SHALL show every message, tool call, and tool result of that child in order, SHALL open the view read-only, and SHALL return to the parent transcript on Esc without changing either history.

#### Scenario: Drill into a finished child
- **WHEN** the user opens the `task` row of a finished child
- **THEN** the view lists the child's messages, its tool calls with arguments and results, and its final text in order

#### Scenario: Return to the parent
- **WHEN** the user presses Esc in a child view
- **THEN** the parent transcript is shown again and the next request is unchanged

#### Scenario: Inspect after resume
- **WHEN** the user resumes a session and opens a child transcript from an earlier turn
- **THEN** the same messages are shown as before the resume

#### Scenario: Running child
- **WHEN** the user opens a child that is still working
- **THEN** the messages so far are shown and new ones appear as they arrive

### Requirement: Cancellation reaches children

The system SHALL stop a running child when the user cancels the turn, and SHALL end the parent turn with the cancelled notice.

#### Scenario: Esc during a child
- **WHEN** the user presses Esc while a child is working
- **THEN** the child's request stops and the turn reports cancelled

### Requirement: A child has a step cap

The system SHALL stop a child after the configured `max_steps` tool rounds and SHALL return its last text with a note that the step cap was reached.

#### Scenario: Step cap
- **WHEN** a child keeps calling tools past `max_steps`
- **THEN** the tool result contains the last text and says the child stopped at the step cap
