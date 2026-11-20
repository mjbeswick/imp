## MODIFIED Requirements

### Requirement: Memory costs nothing until used

The system SHALL describe memory in at most two lines of the system prompt plus an index of at most forty notes, one line each with the note's slug and title, sorted with project notes first, and SHALL return note bodies only from a search result.

#### Scenario: Empty session
- **WHEN** a session starts with eighty notes on disk
- **THEN** the prompt contains the memory pointer and forty index lines, and no note body

#### Scenario: Indexed note
- **WHEN** `.imp/memory/user-location-woking-uk.md` starts with `# User location: Woking, UK`
- **THEN** the prompt contains `user-location-woking-uk: User location: Woking, UK`

#### Scenario: One search
- **WHEN** the model searches for a term
- **THEN** at most five notes come back with short excerpts and the rest cost nothing

### Requirement: The memory tool has two operations

The system SHALL expose a `memory` tool with `search` and `save`, where search matches case-insensitively a note whose name or body contains every word of the query in any order, and save refuses a note whose slug already exists.

#### Scenario: Search hit
- **WHEN** the model searches for a term present in one note's body
- **THEN** that note is returned with a body excerpt and its path

#### Scenario: Words apart
- **WHEN** the model searches for `woking location` and a note contains both words apart
- **THEN** that note is returned

#### Scenario: Duplicate slug
- **WHEN** the model saves a note whose slug already exists
- **THEN** the save is refused and the result names the existing file

#### Scenario: Nothing found
- **WHEN** a search matches no note
- **THEN** the result says so without writing anything
