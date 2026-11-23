# setup Specification

## Purpose
Creates the file layout on first use, with one real example of each thing a user edits, and never touches anything that already exists.

## Requirements

### Requirement: One command creates the layout

The system SHALL provide `imp init`, which creates the config directory, the state directories, and both parent directories for skills, agents, memory, sessions, and spilled output, and prints each path with `created` or `kept`.

#### Scenario: Fresh machine
- **WHEN** the user runs `imp init` with no config present
- **THEN** the directories and two example files are created and each is reported

#### Scenario: Second run
- **WHEN** the user runs it again
- **THEN** nothing is overwritten; every existing path is reported as `kept`

### Requirement: Only two files are written

The system SHALL write exactly one agent example and one skill example, SHALL write a commented config template containing no API key, and SHALL NOT write any session, memory note, or agent other than the examples.

#### Scenario: Template contents
- **WHEN** the config template is created
- **THEN** every meaningful key is present and commented, including `links`, and `api_key` is absent

#### Scenario: Examples are real
- **WHEN** the two example files are read
- **THEN** they are valid files of their kind and would load without edits

### Requirement: Init is offline and silent about secrets

The system SHALL make no network request, SHALL prompt for nothing, SHALL complete in under a second, and SHALL print the next step as a single line.

#### Scenario: Runs in a pipeline
- **WHEN** `imp init` is run with no terminal attached
- **THEN** it completes successfully and prints plain lines with no escape sequences

#### Scenario: No key needed
- **WHEN** `DEEPSEEK_API_KEY` is unset
- **THEN** init still succeeds and says to set it

### Requirement: The agent may read its config but not rewrite it

The system SHALL allow reading every configuration file, SHALL refuse agent writes to the main config file in `normal` and `strict`, and SHALL prompt in `yolo`.

#### Scenario: Agent attempts self-escalation
- **WHEN** the model calls `write` on the main config file
- **THEN** the call is refused with a message saying the user edits it

#### Scenario: Agent adds a skill
- **WHEN** the model writes a new `SKILL.md` under a skills directory
- **THEN** it is treated as an ordinary file write under the permission policy

### Requirement: A run with nothing to do says why

The system SHALL, when a run has no prompt, no interactive terminal, and reads no input, print one line to stderr saying the chat needs a terminal and how to pass a prompt, and SHALL leave the exit code at 0.

#### Scenario: Resumed with no input
- **WHEN** `imp -r` runs with stdin at end of file and no prompt
- **THEN** it prints the resume line, then one line saying the chat needs a terminal, and exits 0

#### Scenario: A pipe with input
- **WHEN** the user runs `echo "fix the parser" | imp`
- **THEN** the prompt runs and no hint is printed

#### Scenario: A prompt argument
- **WHEN** the user runs `imp "fix the parser"` with no terminal
- **THEN** the prompt runs and no hint is printed

### Requirement: Reload re-reads the config

The system SHALL re-read the config file on `/reload`, SHALL apply each key whose value in the file changed since it was last read and list it as `key: old → new`, SHALL keep the session's value for every key the file did not change, SHALL name a key removed from the file as needing a restart, and SHALL name unknown keys once at startup and on reload without failing.

#### Scenario: Privilege setting
- **WHEN** the user adds `allow_sudo=true` to the config and runs `/reload`
- **THEN** a line reads `allow_sudo: unset → true` and the next privileged command is classified as allowed

#### Scenario: Session choice kept
- **WHEN** the user picked a model with `/model`, then edits only `allow_sudo` and reloads
- **THEN** the picked model stays

#### Scenario: Unknown key
- **WHEN** the config contains a key imp does not know
- **THEN** startup names that key as unknown and the run continues
