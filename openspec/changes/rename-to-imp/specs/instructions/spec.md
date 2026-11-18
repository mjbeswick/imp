# Spec Delta

## Purpose

Loads project instructions and file-defined skills and agents from the places people already keep them, so a repository can teach the agent without any new format or daemon.

## MODIFIED Requirements

### Requirement: A compiled baseline always ships

The system SHALL send a compiled baseline prompt on every request covering tool semantics, output caps, path confinement, and when to ask, SHALL allow agent files to add role without replacing it, and SHALL NOT make the baseline depend on any file existing.

#### Scenario: Fresh installation
- **WHEN** no agent file and no `AGENTS.md` exist
- **THEN** the request still carries the baseline with every tool named and both caps stated

#### Scenario: Agent switch
- **WHEN** the agent changes mid-conversation
- **THEN** the baseline is unchanged and only the role note is appended

#### Scenario: Drift is caught
- **WHEN** a tool is added or renamed without the baseline being updated
- **THEN** the build fails a test that checks every tool name appears in the baseline

#### Scenario: Inspectable
- **WHEN** the user runs `imp prompt`
- **THEN** the baseline prints, and nothing else is composed with it

#### Scenario: Budget
- **WHEN** the baseline is measured
- **THEN** it stays under 2,400 bytes so the default request stays near 1.4k tokens

### Requirement: Skills and agents load from the same roots

The system SHALL discover skills in `skills/<name>/SKILL.md` and agents in `agents/<name>.md` under the global config directory, the project's `.imp/` directory, and any extra directory passed at launch, with project entries shadowing global ones by name.

#### Scenario: Project agent
- **WHEN** `.imp/agents/reviewer.md` exists
- **THEN** the agent is available and its file path is shown in the interface

#### Scenario: Shadowing
- **WHEN** a global and a project skill share a name
- **THEN** the project one is used
