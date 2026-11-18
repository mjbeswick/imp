# Spec Delta

## Purpose

Keeps durable facts as plain markdown files that any editor can open, with no frontmatter, no schema, and no standing prompt cost.

## MODIFIED Requirements

### Requirement: Notes live in two directories

The system SHALL read notes from a global directory and, when the session directory contains `.git`, `.jj`, or `.imp`, from the project's `.imp/memory/` directory, and SHALL treat project notes and global notes as one set for search.

#### Scenario: Project note
- **WHEN** the session is inside a jj repository
- **THEN** notes saved there land in `.imp/memory/` and are found by search

#### Scenario: Global note
- **WHEN** the session is outside any repository
- **THEN** notes land in the global memory directory

#### Scenario: Both scopes searched
- **WHEN** the same term appears in a project note and a global note
- **THEN** both appear in one search result

### Requirement: Memory writes follow the permission policy

The system SHALL treat a write, edit, or delete of a memory file as an ordinary file mutation subject to the permission policy, and SHALL NOT apply a separate approval rule for memory.

#### Scenario: Global note save
- **WHEN** the model saves a note to the global memory directory outside the session directory
- **THEN** the write prompts in `normal` mode because it is outside the session directory

#### Scenario: Project note save
- **WHEN** the model saves a note under the project's `.imp/memory/`
- **THEN** the write follows the same rule as any other write inside the session directory
