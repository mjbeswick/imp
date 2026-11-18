# instructions Specification

## Purpose
Loads project instructions and file-defined skills and agents from the places people already keep them, so a repository can teach the agent without any new format or daemon.

## Requirements

### Requirement: AGENTS.md is discovered from three roots

The system SHALL read instructions from a global file, from the repository root's `AGENTS.md`, and from the session directory's `AGENTS.md`, in that order, and SHALL treat `CLAUDE.md` as an alternative only when `AGENTS.md` is absent in the same directory.

#### Scenario: Repository instructions
- **WHEN** the session starts inside a repository with `AGENTS.md` at its root
- **THEN** that file's contents are in the system prompt

#### Scenario: Nested directory
- **WHEN** the session directory has its own `AGENTS.md` below the repository root
- **THEN** both files are included, repository first

#### Scenario: Both names present
- **WHEN** a directory has both `AGENTS.md` and `CLAUDE.md`
- **THEN** only `AGENTS.md` is read

### Requirement: Instructions are bounded

The system SHALL cap loaded instructions at 8 KB total, SHALL mark truncation explicitly, and SHALL report which files were loaded when the session starts.

#### Scenario: Very large file
- **WHEN** `AGENTS.md` is 40 KB
- **THEN** the prompt carries the first 8 KB with a truncation marker

#### Scenario: Load report
- **WHEN** the session starts
- **THEN** the interface lists the instruction files it loaded and their sizes

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

### Requirement: Instructions are part of the frozen prefix

The system SHALL load instructions once at session start and SHALL NOT re-read them during the session unless the user asks to reload.

#### Scenario: Edited while running
- **WHEN** `AGENTS.md` is edited during a session
- **THEN** the change appears only after an explicit reload

#### Scenario: Reload reported
- **WHEN** the user reloads instructions
- **THEN** the interface reports the reload and that the prompt cache was reset

### Requirement: Skills and agents load from the same roots

The system SHALL discover skills in `skills/<name>/SKILL.md` and agents in `agents/<name>.md` under the global config directory, the project's `.imp/` directory, and any extra directory passed at launch, with project entries shadowing global ones by name.

#### Scenario: Project agent
- **WHEN** `.imp/agents/reviewer.md` exists
- **THEN** the agent is available and its file path is shown in the interface

#### Scenario: Shadowing
- **WHEN** a global and a project skill share a name
- **THEN** the project one is used

### Requirement: No files is a valid state

The system SHALL run normally with no instruction, skill, or agent files present, and SHALL NOT create any of them.

#### Scenario: Fresh directory
- **WHEN** the session starts in an empty directory with no global config
- **THEN** the agent runs with the built-in prompt and no errors
