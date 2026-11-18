# Spec Delta

## Purpose

Creates the file layout on first use, with one real example of each thing a user edits, and never touches anything that already exists.

## MODIFIED Requirements

### Requirement: One command creates the layout

The system SHALL provide `imp init`, which creates the config directory, the state directories, and both parent directories for skills, agents, memory, sessions, and spilled output, and prints each path with `created` or `kept`.

#### Scenario: Fresh machine
- **WHEN** the user runs `imp init` with no config present
- **THEN** the directories and two example files are created and each is reported

#### Scenario: Second run
- **WHEN** the user runs it again
- **THEN** nothing is overwritten; every existing path is reported as `kept`

### Requirement: Init is offline and silent about secrets

The system SHALL make no network request, SHALL prompt for nothing, SHALL complete in under a second, and SHALL print the next step as a single line.

#### Scenario: Runs in a pipeline
- **WHEN** `imp init` is run with no terminal attached
- **THEN** it completes successfully and prints plain lines with no escape sequences

#### Scenario: No key needed
- **WHEN** `DEEPSEEK_API_KEY` is unset
- **THEN** init still succeeds and says to set it
