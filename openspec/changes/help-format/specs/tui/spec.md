# Spec Delta

## ADDED Requirements

### Requirement: Help output is laid out in tables

The system SHALL print help in named sections, SHALL align each section's descriptions in one column two spaces after its longest entry, SHALL wrap a description that does not fit with its continuation under the same column, SHALL keep every row inside the requested width, and SHALL use the same layout for `--help`, `/help`, and line mode.

#### Scenario: Aligned sections
- **WHEN** the user runs `imp --help`
- **THEN** every flag in a section starts in one column and every description starts in the next

#### Scenario: Wrapped description
- **WHEN** a flag description is wider than the terminal
- **THEN** it wraps under the description column and the flag row stays whole

#### Scenario: Command sections
- **WHEN** the user runs `/help`
- **THEN** keys and commands appear in separate sections with aligned rows

#### Scenario: Narrow terminal
- **WHEN** the width is 60 columns
- **THEN** no help row is wider than 60 and no flag name is cut

#### Scenario: Same content
- **WHEN** help is printed in the chat, line mode, or with `--help`
- **THEN** the same commands and flags are named in each
