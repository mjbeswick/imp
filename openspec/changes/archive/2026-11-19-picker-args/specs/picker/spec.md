# Spec Delta

## REMOVED Requirements

### Requirement: Directory picker stays inside the session root

**Reason**: The browser now climbs above the session root, so the boundary rule is no longer true.

**Migration**: the replacement requirement below keeps the root as the starting point and allows parent navigation above it.

## ADDED Requirements

### Requirement: Directory picker can climb above the session root

The system SHALL open a directory browser for `/cd` with no argument, SHALL start it at the current session root, SHALL show directories under the browsed directory, SHALL enter the highlighted directory on Enter, SHALL select it on Tab and close, SHALL go to the parent on Left, SHALL allow Left above the session root up to the filesystem root, and SHALL change the root only after the selection succeeds.

#### Scenario: Enter navigates
- **WHEN** the highlight is on `src/` and the user presses Enter
- **THEN** the browser lists `src/` and the session root is unchanged

#### Scenario: Tab selects
- **WHEN** the highlight is on `src/` and the user presses Tab
- **THEN** the picker closes and the session root becomes the absolute path of `src/`

#### Scenario: Parent
- **WHEN** the user presses Left inside `src/`
- **THEN** the browser returns to the directory above it

#### Scenario: Above the root
- **WHEN** the browser is at the session root and the user presses Left
- **THEN** it lists the parent directory, and Tab there makes that parent the session root

#### Scenario: Filesystem root
- **WHEN** the browser is at `/` and the user presses Left
- **THEN** nothing moves

#### Scenario: Typed path
- **WHEN** the user runs `/cd ~/Projects/imp`
- **THEN** the root changes directly and no browser opens

### Requirement: The picker filter takes focus and shows a cursor

The system SHALL put the keyboard focus in the picker filter while the overlay is open, SHALL draw a visible cursor at the end of the filter text, SHALL append typed characters and remove them with Backspace, SHALL hide the cursor again when the overlay closes, and SHALL leave the terminal unchanged when the overlay does not fit.

#### Scenario: Cursor
- **WHEN** the picker opens with an empty filter
- **THEN** a cursor is visible in the filter line

#### Scenario: Typing
- **WHEN** the user types `rev`
- **THEN** the filter reads `rev` with the cursor after it and the list narrows

#### Scenario: Backspace
- **WHEN** the filter is `rev` and the user presses Backspace
- **THEN** the filter reads `re` and the cursor sits after `e`

#### Scenario: Close
- **WHEN** the picker closes
- **THEN** the cursor returns to the composer and no cursor remains in the overlay region

## MODIFIED Requirements

### Requirement: One picker serves command values

The system SHALL provide a single picker with a title, a filter line, a scrolling list, a highlighted row, and a cancel action, SHALL open it for `/agent`, `/sessions`, `/verbosity`, and `/model` when no value is given, SHALL open it after a valid command word and a space when that command has enumerable values, SHALL apply a typed value directly without opening it, SHALL filter case-insensitively as the user types, SHALL move with Up/Down and page with PgUp/PgDn, SHALL choose with Enter, and SHALL cancel with Esc without changing anything.

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
- **THEN** the picker lists `low` and `high` and choosing one applies it

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
