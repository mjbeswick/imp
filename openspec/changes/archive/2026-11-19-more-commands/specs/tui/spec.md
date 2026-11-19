# Spec Delta

## REMOVED Requirements

### Requirement: Fifteen slash commands, no more

**Reason**: `/shell`, `/model`, `/provider`, and `/stash` are the sixteenth through nineteenth commands.

**Migration**: the nineteen-command requirement below replaces it; `/clear`, `/model` as a config command, `/mode`, and `/cost` remain absent.

## ADDED Requirements

### Requirement: Nineteen slash commands, no more

The system SHALL provide exactly `/help`, `/new`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/yolo`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/stash`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/mode`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all nineteen commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

### Requirement: A shell can be opened from the prompt

The system SHALL suspend the full-screen interface, restore cooked input, and run `$SHELL` for `/shell`, SHALL restore the interface and redraw when it exits, and SHALL run `$SHELL` directly in line mode.

#### Scenario: Chat shell
- **WHEN** the user runs `/shell` in chat
- **THEN** the normal shell prompt appears and returning to it redraws the chat

#### Scenario: Line mode shell
- **WHEN** the user runs `/shell` in line mode
- **THEN** the shell runs and returns to the prompt

#### Scenario: No shell variable
- **WHEN** `$SHELL` is unset
- **THEN** `/bin/sh` is used

### Requirement: The session model can be chosen

The system SHALL set the session model for `/model <name>`, SHALL open a picker for bare `/model` listing the current model, the provider default, and recently used models, SHALL record a chosen model in the recents state, and SHALL NOT rewrite the config file.

#### Scenario: Direct
- **WHEN** the user runs `/model deepseek-v4-pro`
- **THEN** the next request uses that model

#### Scenario: Picker
- **WHEN** the user runs `/model` with no argument
- **THEN** the picker lists the current, default, and recent models

#### Scenario: Recents
- **WHEN** a model is chosen
- **THEN** it appears at the front of the next picker

### Requirement: Provider profiles can be managed

The system SHALL list built-in and custom providers for bare `/provider`, SHALL support `add`, `edit`, `delete`, and `use`, SHALL store custom profiles in a managed section of the config file while preserving every other line, SHALL require a base URL, model, and dialect for a profile, and SHALL refuse a delete that leaves the active provider missing.

#### Scenario: Add
- **WHEN** the user runs `/provider add work` and supplies base URL, model, and dialect
- **THEN** the config gains a `provider.work.*` block and `work` lists next time

#### Scenario: Edit
- **WHEN** the user edits `work`
- **THEN** only its managed lines change and comments survive

#### Scenario: Delete
- **WHEN** the user deletes a profile that is not active
- **THEN** its lines are removed

#### Scenario: Delete active
- **WHEN** the user deletes the active profile
- **THEN** the command refuses and asks them to switch first

#### Scenario: Use
- **WHEN** the user runs `/provider use work`
- **THEN** the active provider becomes `work` for the session

### Requirement: Messages can be stashed for the session

The system SHALL keep a session-bound stash of messages, SHALL add text with `/stash <text>`, SHALL list entries numbered for bare `/stash`, SHALL load entry `n` into the composer and remove it for `pop n`, SHALL remove it without loading for `delete n`, SHALL load all entries joined by blank lines for `pop all`, and SHALL NOT send a request to the model.

#### Scenario: Add while busy
- **WHEN** the agent is streaming and the user runs `/stash check the tests`
- **THEN** the text is stored and the turn is not interrupted

#### Scenario: List
- **WHEN** the user runs `/stash`
- **THEN** each entry appears with its number

#### Scenario: Pop into composer
- **WHEN** the user runs `/stash pop 2`
- **THEN** entry 2 is removed and its text sits in the composer, ready to edit

#### Scenario: Pop all
- **WHEN** the user runs `/stash pop all`
- **THEN** every entry is removed and joined into the composer

#### Scenario: Delete
- **WHEN** the user runs `/stash delete 1`
- **THEN** entry 1 is removed and the composer is unchanged
