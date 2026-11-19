# Spec Delta

## MODIFIED Requirements

### Requirement: An agent is a markdown file

The system SHALL load agents from `agents/<name>.md` under an explicit extra directory, the project `.agents/`, the project `.imp/`, and the global config directory, in that order of precedence, with the file's first heading or filename as its name, its body as instructions, and optional frontmatter keys `tools`, `model`, and `thinking`. It SHALL skip entries that are not `.md` files and SHALL prefer the earlier source when two files share a name.

#### Scenario: Project agent
- **WHEN** `.imp/agents/reviewer.md` exists
- **THEN** it appears in the agent list and its body joins the instructions

#### Scenario: Shared agents directory
- **WHEN** `.agents/agents/reviewer.md` exists
- **THEN** `reviewer` appears in the list with that file's instructions

#### Scenario: Shadowed profile
- **WHEN** `reviewer.md` exists in both `.imp/agents` and `.agents/agents`
- **THEN** the `.agents` version is the one listed

#### Scenario: Stray files
- **WHEN** `.agents/agents/README.txt` exists
- **THEN** it is ignored

#### Scenario: Restricted tools
- **WHEN** an agent sets `tools: read, bash`
- **THEN** `write` and `edit` are absent from its tool list

#### Scenario: No agent files
- **WHEN** no agent files exist
- **THEN** a built-in `default` profile is available with all tools and no extra instructions

### Requirement: The agent is selectable at launch and switchable later

The system SHALL accept `--agent <name>` at launch, SHALL open a picker for `/agent` with no argument, SHALL switch directly for `/agent <name>`, SHALL report the active profile for `/agent current`, SHALL list the available names for an unknown name, and SHALL keep the conversation history when the agent changes.

#### Scenario: Launch with a profile
- **WHEN** the user runs `imp --agent explore`
- **THEN** the session starts with that profile's instructions and tools

#### Scenario: Switch mid-conversation
- **WHEN** the user picks `reviewer` during a conversation
- **THEN** the next request uses the new instructions and tools, and the transcript above is unchanged

#### Scenario: Switch back
- **WHEN** the user switches to another agent and then back
- **THEN** the original agent's instructions and tools apply again

#### Scenario: Direct switch
- **WHEN** the user runs `/agent reviewer`
- **THEN** the session switches to `reviewer` without opening the picker

#### Scenario: Current profile
- **WHEN** the user runs `/agent current`
- **THEN** the transcript names the profile, its model, its thinking setting, and its tools

#### Scenario: Unknown name
- **WHEN** the user runs `/agent nope`
- **THEN** the transcript reports that no such profile exists and lists the available names
