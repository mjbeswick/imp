# Spec Delta

## Purpose

Defines agents as named profiles — instructions, tool subset, model, thinking — selectable at launch and switchable mid-conversation without losing the history.

## MODIFIED Requirements

### Requirement: An agent is a markdown file

The system SHALL load agents from `agents/<name>.md` under the global config directory, the project's `.imp/` directory, and any extra directory passed at launch, with the file's first heading or filename as its name, its body as instructions, and optional frontmatter keys `tools`, `model`, and `thinking`.

#### Scenario: Project agent
- **WHEN** `.imp/agents/reviewer.md` exists
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
- **WHEN** the user runs `imp --agent explore`
- **THEN** the session starts with that profile's instructions and tools

#### Scenario: Switch mid-conversation
- **WHEN** the user picks `reviewer` during a conversation
- **THEN** the next request uses the new instructions and tools, and the transcript above is unchanged

#### Scenario: Switch back
- **WHEN** the user switches to another agent and then back
- **THEN** the original agent's instructions and tools apply again
