# Spec Delta

## Purpose

Lets the running agent delegate a scoped piece of work to a second agent with its own history, its own tool subset, and no ability to delegate further.

## ADDED Requirements

### Requirement: Agents are defined by name and tool set

The system SHALL load agent definitions from files with frontmatter naming the agent, describing it, listing its tools, and optionally setting a model, and SHALL provide `general` and `explore` without any files present.

#### Scenario: Built-in fallback
- **WHEN** no agent directory exists
- **THEN** `general` and `explore` are available

#### Scenario: Restricted agent
- **WHEN** an agent definition lists only read tools
- **THEN** a run of that agent has no write or edit tool available

### Requirement: Delegation runs with fresh history

The system SHALL provide a `task` operation taking an agent name and a prompt, running it in a fresh conversation with the parent's working directory and path jail, and returning only the child's final text, capped at 4 KB.

#### Scenario: Parent history stays clean
- **WHEN** a child performs eight tool calls
- **THEN** the parent's history contains one tool result, not the child's transcript

#### Scenario: Unknown agent
- **WHEN** the model names an agent that does not exist
- **THEN** the result lists the available agents

#### Scenario: No arguments
- **WHEN** the model calls `task` without an agent
- **THEN** the result lists available agents with their descriptions

### Requirement: Delegation does not nest

The system SHALL remove the `task` tool from any child agent, so a child cannot start a further child.

#### Scenario: Child attempts delegation
- **WHEN** a child model requests `task`
- **THEN** the call is unavailable and the child answers without it

### Requirement: Child questions reach the user

The system SHALL route a confirmation or an `ask` question requested by a child to the interface, tagged with the child's name, and SHALL block the child until the user answers.

#### Scenario: Child wants to delete
- **WHEN** a child agent requests a destructive command
- **THEN** the prompt appears once in the interface naming the child, and the answer applies to that request

#### Scenario: Child asks a question
- **WHEN** a child agent calls `ask`
- **THEN** the picker appears tagged with the child's name and its answer returns to the child

### Requirement: Cancellation reaches children

The system SHALL cancel every running child when the parent turn is cancelled, and SHALL report cancellation in the parent's tool result.

#### Scenario: Cancel during delegation
- **WHEN** the user cancels while a child is working
- **THEN** the child stops before its next tool call and the parent turn ends as cancelled
