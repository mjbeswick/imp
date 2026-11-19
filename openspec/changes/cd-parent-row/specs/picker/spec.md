# Spec Delta

## ADDED Requirements

### Requirement: The directory list starts with the parent row

The system SHALL put a `../` row first in every directory browser listing that
has a parent, SHALL omit it at the filesystem root, SHALL open the parent on
Enter, SHALL select the parent as the session root on Tab, and SHALL leave the
image browser without the row.

#### Scenario: Top row
- **WHEN** the user opens `/cd` at the session root
- **THEN** `../` is the first row, above the session's own directories

#### Scenario: Enter
- **WHEN** the highlight is on `../` and the user presses Enter
- **THEN** the browser lists the parent directory and the session root is unchanged

#### Scenario: Tab
- **WHEN** the highlight is on `../` and the user presses Tab
- **THEN** the picker closes and the parent becomes the session root

#### Scenario: Filesystem root
- **WHEN** the browser lists `/`
- **THEN** no `../` row appears

#### Scenario: Image browser
- **WHEN** the user opens `/image`
- **THEN** no `../` row appears and Left keeps its existing behaviour
