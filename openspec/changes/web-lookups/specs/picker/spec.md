## MODIFIED Requirements

### Requirement: One picker serves command values

The system SHALL provide a single picker with a title, a filter, a scrolling list, a highlighted row, and cancel, SHALL open it in the chat for `/agent`, `/sessions`, `/verbosity`, `/model`, `/theme`, `/provider`, and `/mode` with no value, and after a command word and space when that command has values, SHALL apply a typed value directly, SHALL filter case-insensitively, SHALL move with Up/Down and page with PgUp/PgDn, SHALL choose with Enter, and SHALL cancel with Esc unchanged.

#### Scenario: Agent picker
- **WHEN** the user runs `/agent`
- **THEN** the profile list opens with the current profile highlighted

#### Scenario: Value after the word
- **WHEN** the user types `/agent ` and then `rev`
- **THEN** the agent picker opens, the filter holds `rev`, and the profiles narrow

#### Scenario: Typed value
- **WHEN** the user runs `/agent reviewer`
- **THEN** the session switches directly and no picker opens

#### Scenario: Verbosity picker
- **WHEN** the user runs `/verbosity` with no level
- **THEN** the picker lists `low`, `medium`, and `high` and choosing one applies it

#### Scenario: Theme and provider pickers
- **WHEN** the user runs `/theme` or `/provider` with no value in the chat
- **THEN** the picker opens with the current entry highlighted

#### Scenario: Mode picker
- **WHEN** the user runs `/mode` with no value in the chat
- **THEN** the picker lists `normal`, `strict`, and `yolo` with a description each, opens on the current mode, and marks it `current`

#### Scenario: Subcommand with an argument
- **WHEN** the user types `/provider add work` and presses Enter
- **THEN** the picker closes at the space after `add`, the composer holds the full line, and Enter runs it

#### Scenario: Filter
- **WHEN** the user types `rev` in the agent picker
- **THEN** only profiles whose names contain `rev` remain and the highlight moves to the first

#### Scenario: Cancel
- **WHEN** the user presses Esc
- **THEN** the picker closes and the session is unchanged

#### Scenario: Free text stays in the composer
- **WHEN** the user types `/stash ` or `/rename `
- **THEN** no picker opens and the composer keeps the text

#### Scenario: Empty result
- **WHEN** the filter matches nothing
- **THEN** the list shows a no-match row and Enter does nothing
