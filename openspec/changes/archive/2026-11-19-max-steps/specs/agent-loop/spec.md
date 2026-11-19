# Spec Delta

## MODIFIED Requirements

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
