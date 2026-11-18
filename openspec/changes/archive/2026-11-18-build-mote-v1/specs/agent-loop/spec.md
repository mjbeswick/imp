# Spec Delta

## Purpose

Turns one user request into a completed answer by streaming a completion, running whatever tools the model asks for, and continuing until the model stops asking. Owns retries, cancellation, and thinking mode.

## ADDED Requirements

### Requirement: A turn ends only when the model stops calling tools

The system SHALL repeat completion and tool execution until the model returns a response with no tool calls, and SHALL stop after 24 tool iterations with a visible error.

#### Scenario: Tool then answer
- **WHEN** the model returns two tool calls and then a text response
- **THEN** both tools run, their results are sent back, and the text response ends the turn

#### Scenario: Runaway loop
- **WHEN** the model calls tools for the 24th consecutive time
- **THEN** the loop stops and the transcript shows that the iteration limit was reached

### Requirement: Streamed tool calls are assembled by index

The system SHALL accumulate tool-call fragments by their `index` field, taking the identifier and function name from the first fragment and concatenating arguments from all fragments, ignoring later fragments that carry an empty name.

#### Scenario: Split arguments
- **WHEN** one tool call arrives as four fragments with arguments split across them
- **THEN** the system produces one call whose arguments parse as valid JSON

#### Scenario: Parallel calls
- **WHEN** two tool calls interleave with indexes 0 and 1
- **THEN** both are reconstructed separately and neither contains fragments from the other

### Requirement: Transient failures are retried once

The system SHALL retry a request once after a delay not exceeding two seconds when it fails with `insufficient_system_resource`, `aborted`, a 429, a 5xx, or a transport error, and SHALL surface the error if the retry also fails.

#### Scenario: Overloaded upstream
- **WHEN** the first request ends with `finish_reason` `insufficient_system_resource`
- **THEN** the request is issued again once and the turn continues

#### Scenario: Repeated failure
- **WHEN** the retried request fails the same way
- **THEN** the turn ends with an inline error naming the failure

### Requirement: Cancellation is immediate

The system SHALL cancel the active turn within one second of the cancel key, terminate any running tool process group within two seconds, and report the turn as cancelled.

#### Scenario: Cancel while streaming
- **WHEN** the user presses Esc while tokens are arriving
- **THEN** the stream stops, partial text remains visible, and the input is ready

#### Scenario: Cancel a running command
- **WHEN** the user presses Esc while a long `bash` command runs
- **THEN** the command's process group receives SIGTERM, then SIGKILL after two seconds, and the tool result reads cancelled

### Requirement: Thinking mode is off by default

The system SHALL send `thinking` disabled by default, SHALL enable it for a single turn or for the session on request, and SHALL send `reasoning_content` back for every prior assistant message whenever tools are present.

#### Scenario: Default request
- **WHEN** a request is sent with no thinking override
- **THEN** it carries `{"thinking":{"type":"disabled"}}` and no reasoning is displayed

#### Scenario: Thinking enabled for one turn
- **WHEN** the user enables thinking and sends a message
- **THEN** only that turn is sent with thinking enabled and reasoning is displayed dimmed

#### Scenario: Reasoning echoed
- **WHEN** a previous assistant message in the history contains reasoning content
- **THEN** the next request includes that reasoning content unchanged

### Requirement: Every finish reason has a defined outcome

The system SHALL treat `stop` and `tool_calls` as success, retry `aborted` and `insufficient_system_resource`, and report `length`, `content_filter`, and unknown reasons as errors with their own message.

#### Scenario: Output cap hit
- **WHEN** the response ends with `finish_reason` `length`
- **THEN** the transcript says the reply was truncated and offers retry with a larger limit
