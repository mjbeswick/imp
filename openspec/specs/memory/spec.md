# memory Specification

## Purpose
Keeps durable facts as plain markdown files that any editor can open, with no frontmatter, no schema, and no standing prompt cost.

## Requirements

### Requirement: A note is one markdown file

The system SHALL store each note as a single `.md` file whose name is a slug of its title, whose first heading is the title, and whose body is ordinary markdown, and SHALL NOT require frontmatter or any metadata block.

#### Scenario: Hand-written note
- **WHEN** a user creates `rsync-flags.md` by hand in a memory directory with a heading and prose
- **THEN** the system reads it as a note with no conversion step

#### Scenario: Note without a heading
- **WHEN** a file has no heading
- **THEN** its title is derived from the filename and the file is still usable

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

### Requirement: Memory costs nothing until used

The system SHALL describe memory in at most two lines of the system prompt, SHALL NOT include a note index, and SHALL return note bodies only from a search result.

#### Scenario: Empty session
- **WHEN** a session starts with eighty notes on disk
- **THEN** the prompt contains the memory pointer and none of the notes

#### Scenario: One search
- **WHEN** the model searches for a term
- **THEN** at most five notes come back with short excerpts and the rest cost nothing

### Requirement: The memory tool has two operations

The system SHALL expose a `memory` tool with `search` and `save`, where search matches case-insensitively across filename and body, and save refuses a note whose slug already exists.

#### Scenario: Search hit
- **WHEN** the model searches for a term present in one note's body
- **THEN** that note is returned with a body excerpt and its path

#### Scenario: Duplicate slug
- **WHEN** the model saves a note whose slug already exists
- **THEN** the save is refused and the result names the existing file

#### Scenario: Nothing found
- **WHEN** a search matches no note
- **THEN** the result says so without writing anything

### Requirement: Memory files are plain enough to grep

The system SHALL make every note readable by ordinary tools, and SHALL NOT depend on any file being present in a special order, encoding, or index.

#### Scenario: External edit
- **WHEN** a note is edited outside the agent
- **THEN** the next search sees the edit with no reload

#### Scenario: External deletion
- **WHEN** a note is deleted outside the agent
- **THEN** it stops appearing in search results

### Requirement: Memory writes follow the permission policy

The system SHALL treat a write, edit, or delete of a memory file as an ordinary file mutation subject to the permission policy, and SHALL NOT apply a separate approval rule for memory.

#### Scenario: Global note save
- **WHEN** the model saves a note to the global memory directory outside the session directory
- **THEN** the write prompts in `normal` mode because it is outside the session directory

#### Scenario: Project note save
- **WHEN** the model saves a note under the project's `.imp/memory/`
- **THEN** the write follows the same rule as any other write inside the session directory

### Requirement: Durable facts are captured at compaction

The system SHALL ask the summarizer to emit up to three durable facts, and SHALL offer each as a `save` proposal rather than writing it.

#### Scenario: Compaction finds a preference
- **WHEN** compaction runs over a conversation where the user stated a lasting preference
- **THEN** the interface offers to save it and writes nothing without approval
