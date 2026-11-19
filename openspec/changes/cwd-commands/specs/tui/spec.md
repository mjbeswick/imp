# Spec Delta

## REMOVED Requirements

### Requirement: Ten slash commands, no more

**Reason**: `/cwd` and `/cd` are the eleventh and twelfth commands.

**Migration**: the twelve-command requirement below replaces it; `/clear`, `/model`, `/reload` as a mode, `/mode`, and `/cost` remain absent.

## ADDED Requirements

### Requirement: Twelve slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/yolo`, `/verbosity`, `/cwd`, `/cd`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/model`, `/reload` as a mode, `/mode`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twelve commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Command effect
- **WHEN** the user runs `/compact`
- **THEN** compaction runs immediately and the result is reported in the transcript

#### Scenario: Reload
- **WHEN** the user runs `/reload` after editing `AGENTS.md`
- **THEN** the next request carries the new instructions and the interface says the cache was reset

#### Scenario: Yolo on
- **WHEN** the user runs `/yolo` in a session launched in `normal`
- **THEN** the mode becomes `yolo`, the path jail turns off, the transcript names both, and the config file is untouched

#### Scenario: Yolo off
- **WHEN** the user runs `/yolo` again in that session
- **THEN** the mode returns to `normal` and the path jail returns

#### Scenario: Duplicate affordance is absent
- **WHEN** the user types `/clear`
- **THEN** it is not a command; Ctrl+L clears the view

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

### Requirement: The session root can be inspected and changed

The system SHALL print the session root for `/cwd`, SHALL change it for `/cd <path>`, SHALL expand a leading `~`, SHALL resolve a relative path against the current root, SHALL refuse a missing path or a file without changing anything, SHALL keep the conversation, SHALL reload instructions, skills, memory, and the system prompt, and SHALL report the new root and the cache reset.

#### Scenario: Show the root
- **WHEN** the user runs `/cwd`
- **THEN** the transcript shows the absolute path the session is rooted at

#### Scenario: Change the root
- **WHEN** the user runs `/cd /Users/michael/Projects/imp`
- **THEN** the transcript names the new root and says the prompt cache was reset

#### Scenario: Tilde
- **WHEN** the user runs `/cd ~/Projects`
- **THEN** the root becomes the home directory joined with `Projects`

#### Scenario: Relative path
- **WHEN** the user runs `/cd ..` from `~/Projects/imp`
- **THEN** the root becomes `~/Projects`

#### Scenario: Missing path
- **WHEN** the user runs `/cd /nope`
- **THEN** the transcript reports that the directory does not exist and the root is unchanged

#### Scenario: A file
- **WHEN** the user runs `/cd Cargo.toml`
- **THEN** the transcript reports that the path is not a directory and the root is unchanged

#### Scenario: Bare command
- **WHEN** the user runs `/cd` with no path
- **THEN** the transcript shows the usage `cd <path>` and nothing changes

#### Scenario: History kept
- **WHEN** the root changes after several turns
- **THEN** the conversation is still present and the next request carries the new instructions

#### Scenario: No model request
- **WHEN** either command runs
- **THEN** nothing is sent to the model
