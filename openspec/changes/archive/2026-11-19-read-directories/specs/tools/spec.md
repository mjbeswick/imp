# Spec Delta

## MODIFIED Requirements

### Requirement: Reads are batched and numbered

The system SHALL accept one or more paths in a single `read` call, SHALL return each file with line numbers and default to 2,000 lines per file, SHALL list a directory as a sorted tree with directories marked, SHALL default to one level of depth and accept at most three, SHALL cap each directory at 200 entries and say when it cut the list, SHALL skip `.git`, `.jj`, `target`, `node_modules`, `dist`, `build`, and `.venv`, and SHALL state clearly when a path is missing or not valid UTF-8.

#### Scenario: Three files at once
- **WHEN** the model reads three paths in one call
- **THEN** all three appear in one result and no second call is needed

#### Scenario: Directory listing
- **WHEN** the model reads `src/`
- **THEN** the result lists its entries in name order, each directory marked with a trailing `/`, and no file contents

#### Scenario: Depth
- **WHEN** the model reads `src/` with `depth: 2`
- **THEN** immediate entries and their children appear, nested by one indent per level

#### Scenario: Default depth
- **WHEN** the model reads a directory without `depth`
- **THEN** only its immediate entries appear

#### Scenario: Noise skipped
- **WHEN** the directory contains `.git`, `target`, and `node_modules`
- **THEN** none of them appear in the listing

#### Scenario: Mixed call
- **WHEN** the model reads a directory and a file in one call
- **THEN** the listing and the numbered file both appear

#### Scenario: Long directory
- **WHEN** a directory holds 500 entries
- **THEN** the first 200 appear in name order with a note that the rest were cut

#### Scenario: Large file
- **WHEN** a file has 10,000 lines and no range is given
- **THEN** the first 2,000 lines are returned with a note on how to continue

#### Scenario: Binary file
- **WHEN** the path is not valid UTF-8
- **THEN** the result says so and does not include mojibake
