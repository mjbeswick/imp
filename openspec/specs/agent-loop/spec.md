# agent-loop Specification

## Purpose
Turns one user request into a completed answer by streaming a completion, running whatever tools the model asks for, and continuing until the model stops asking. Owns retries, cancellation, and thinking mode.

## Requirements

### Requirement: A turn ends only when the model stops calling tools

The system SHALL repeat completion and tool execution until the model returns a response with no tool calls, SHALL stop after the configured number of tool iterations, defaulting to 24, and SHALL end that turn with a visible note that the limit was reached, keeping the completed tool results and the partial answer. It SHALL reject a configured limit below 1 and SHALL cap it at 1,000.

#### Scenario: Tool then answer
- **WHEN** the model returns two tool calls and then a text response
- **THEN** both tools run, their results are sent back, and the text response ends the turn

#### Scenario: Runaway loop
- **WHEN** the model calls tools for the configured number of consecutive iterations
- **THEN** the last batch runs, the turn stops with a note, and every tool call has a matching result

#### Scenario: Custom limit
- **WHEN** `max_steps=2` and the model keeps calling tools
- **THEN** two completion requests are made and the turn pauses

#### Scenario: Resume after the limit
- **WHEN** the user sends another message after a paused turn
- **THEN** the conversation continues with the prior results intact

#### Scenario: Refused value
- **WHEN** the config, environment, or flag sets `max_steps` to 0
- **THEN** imp exits with an error naming the minimum of 1

#### Scenario: Upper bound
- **WHEN** `max_steps` is 5000
- **THEN** it is clamped to 1000 rather than accepted

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

The system SHALL cancel the active turn within one second of the cancel key, SHALL abort an HTTP request in flight whether or not bytes are arriving, SHALL NOT retry a cancelled request, SHALL terminate any running tool process group within two seconds, and SHALL report the turn as cancelled.

#### Scenario: Cancel while streaming
- **WHEN** the user presses Esc while tokens are arriving
- **THEN** the stream stops, partial text remains visible, and the input is ready

#### Scenario: Cancel a running command
- **WHEN** the user presses Esc while a long `bash` command runs
- **THEN** the command's process group receives SIGTERM, then SIGKILL after two seconds, and the tool result reads cancelled

#### Scenario: Cancel while stalled
- **WHEN** the connection stops sending and the user cancels
- **THEN** the request ends within one second and the turn reads cancelled

#### Scenario: Not retried
- **WHEN** a request is cancelled
- **THEN** it is not sent again

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

### Requirement: A stalled stream ends

The system SHALL end a request that receives less than one byte a second for 60 seconds, and SHALL treat it as a transport error.

#### Scenario: Stall
- **WHEN** the provider stops sending mid-reply for a minute
- **THEN** the request ends with a transport error and is retried once

### Requirement: A suggestion follows each turn

The system SHALL, when `suggest` is on and a chat turn ends without error or cancel, make one request of at most 24 output tokens for the user's likely next message, SHALL NOT add it to the history or the session log, SHALL count its tokens in the cost, SHALL drop a reply that is empty, multi-line, or over 60 characters, and SHALL end a suggestion that asks a question with `?`.

#### Scenario: After a question
- **WHEN** a turn ends with the model asking whether to run the tests
- **THEN** a short suggestion such as `yes, run them` reaches the chat and the history is unchanged

#### Scenario: Off
- **WHEN** the config holds `suggest = off`
- **THEN** no suggestion request is made

#### Scenario: Line mode
- **WHEN** imp runs in line mode
- **THEN** no suggestion request is made

#### Scenario: Question mark
- **WHEN** the reply is `should I commit this` or `should I commit this.`
- **THEN** the suggestion is `should I commit this?`

#### Scenario: Not a question
- **WHEN** the reply is `run the tests`
- **THEN** the suggestion is unchanged

### Requirement: A goal runs over several turns

The system SHALL, while a goal is set, start another turn after each turn whose reply's last line is neither `GOAL MET` nor `GOAL BLOCKED`, with a note restating the goal and the round, up to `goal_rounds` rounds (default 5, at most 20), and SHALL end the goal when it is met, blocked, out of rounds, cancelled, interrupted by a user message, refused at a confirmation, or failed with an error.

#### Scenario: Met in two rounds
- **WHEN** the first round's reply ends without a marker and the second ends with `GOAL MET`
- **THEN** exactly two turns run and an info row says the goal was met

#### Scenario: Out of rounds
- **WHEN** `goal_rounds = 2` and no reply carries a marker
- **THEN** two turns run and an info row says the rounds ran out

#### Scenario: Blocked
- **WHEN** a reply ends with `GOAL BLOCKED: needs the API key`
- **THEN** no further round starts and the reason is shown

#### Scenario: Declined
- **WHEN** the user declines a confirmation during a round
- **THEN** no further round starts

#### Scenario: Mentioned, not claimed
- **WHEN** a reply says "I will write GOAL MET when done" in its middle and ends otherwise
- **THEN** another round starts
