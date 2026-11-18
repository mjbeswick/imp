# Spec Delta

## Purpose

Defines agents as named profiles — instructions, tool subset, model, thinking — selectable at launch and switchable mid-conversation without losing the history.

## ADDED Requirements

### Requirement: An agent is a markdown file

The system SHALL load agents from `agents/<name>.md` under the global config directory, the project's `.mote/` directory, and any extra directory passed at launch, with the file's first heading or filename as its name, its body as instructions, and optional frontmatter keys `tools`, `model`, and `thinking`.

#### Scenario: Project agent
- **WHEN** `.mote/agents/reviewer.md` exists
- **THEN** it appears in the agent list and its body joins the instructions

#### Scenario: Restricted tools
- **WHEN** an agent sets `tools: read, bash`
- **THEN** `write` and `edit` are absent from its tool list

#### Scenario: No agent files
- **WHEN** no agent files exist
- **THEN** a built-in `default` profile is available with all tools and no extra instructions

### Requirement: The agent is selectable at launch and switchable later

The system SHALL accept `--agent <name>` at launch, SHALL provide an agent picker during a conversation, and SHALL keep the conversation history when the agent changes.

#### Scenario: Launch with a profile
- **WHEN** the user runs `mote --agent explore`
- **THEN** the session starts with that profile's instructions and tools

#### Scenario: Switch mid-conversation
- **WHEN** the user picks `reviewer` during a conversation
- **THEN** the next request uses the new instructions and tools, and the transcript above is unchanged

#### Scenario: Switch back
- **WHEN** the user switches to another agent and then back
- **THEN** the original agent's instructions and tools apply again

### Requirement: Switching does not rewrite the prefix

The system SHALL deliver a switch as a `system` message at the tail saying that it replaces the previous role instructions, SHALL NOT rewrite messages already in the history, and SHALL report that no cache reset occurred.

#### Scenario: Cache survives
- **WHEN** the agent changes between two requests
- **THEN** the bytes before the conversation tail are identical, apart from the appended note

#### Scenario: Old instructions remain visible
- **WHEN** a switch happens
- **THEN** the previous instructions are still in the history and the new note says it supersedes them

### Requirement: A switch is recorded

The system SHALL record the agent change in the session log and SHALL restore the active agent when the session is resumed.

#### Scenario: Resume
- **WHEN** a session is resumed after a switch
- **THEN** the last selected agent is active and the switch is visible in the transcript

### Requirement: Agents cannot relax permissions

The system SHALL apply the session's permission policy to every agent, and SHALL NOT allow an agent file to change modes, the jail, or the privilege setting.

#### Scenario: Agent tries to widen access
- **WHEN** an agent file contains `permissions: yolo`
- **THEN** the key is ignored and the session policy still applies
