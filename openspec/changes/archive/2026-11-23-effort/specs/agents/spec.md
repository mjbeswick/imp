## MODIFIED Requirements

### Requirement: An agent is a markdown file

The system SHALL load agents from `agents/<name>.md` under an explicit extra directory, the project `.agents/`, the project `.imp/`, and the global config directory, in that order of precedence, with the file's first heading or filename as its name, its body as instructions, and optional frontmatter keys `tools`, `model`, `thinking`, and `effort`. It SHALL skip entries that are not `.md` files and SHALL prefer the earlier source when two files share a name.

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

#### Scenario: Effort key
- **WHEN** an agent file's frontmatter has `effort: high`
- **THEN** switching to that agent sets the session effort to `high`
