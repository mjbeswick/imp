# Spec Delta

## ADDED Requirements

### Requirement: One picker serves command values

The system SHALL provide a single picker with a title, a filter line, a scrolling list, a highlighted row, and a cancel action, SHALL open it for `/agent`, `/sessions`, and `/verbosity` when no value is given, SHALL apply a typed value directly without opening it, SHALL filter case-insensitively as the user types, SHALL move with Up/Down and page with PgUp/PgDn, SHALL choose with Enter, and SHALL cancel with Esc without changing anything.

#### Scenario: Agent picker
- **WHEN** the user runs `/agent`
- **THEN** the profile list opens with the current profile highlighted

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

#### Scenario: Empty result
- **WHEN** the filter matches nothing
- **THEN** the list shows a no-match row and Enter does nothing

### Requirement: Directory picker stays inside the session root

The system SHALL open a directory browser for `/cd` with no argument, SHALL show directories under the current session root with parent navigation, SHALL enter the highlighted directory on Enter, SHALL select it on Tab and close, SHALL go to the parent on Left, SHALL refuse to go above the session root, and SHALL change the root only after the selection succeeds.

#### Scenario: Enter navigates
- **WHEN** the highlight is on `src/` and the user presses Enter
- **THEN** the browser lists `src/` and the session root is unchanged

#### Scenario: Tab selects
- **WHEN** the highlight is on `src/` and the user presses Tab
- **THEN** the picker closes and the session root becomes the absolute path of `src/`

#### Scenario: Parent
- **WHEN** the user presses Left inside `src/`
- **THEN** the browser returns to the directory above while staying at or below the session root

#### Scenario: Root boundary
- **WHEN** the browser is at the session root and the user presses Left
- **THEN** nothing moves

#### Scenario: Typed path
- **WHEN** the user runs `/cd ~/Projects/imp`
- **THEN** the root changes directly and no browser opens

### Requirement: Image picker attaches a file

The system SHALL open a browser for `/image` with no argument, SHALL show directories and PNG, JPEG, GIF, and WebP files under the session root, SHALL mark directories as navigable, SHALL attach the highlighted image on Enter, and SHALL leave the browser open when Enter lands on a directory.

#### Scenario: Browse and attach
- **WHEN** the user runs `/image`, opens `Pictures`, highlights `shot.png`, and presses Enter
- **THEN** the picker closes and the image is attached to the next message

#### Scenario: Non-image file
- **WHEN** a `.txt` file is present
- **THEN** it is not listed

#### Scenario: Directory
- **WHEN** the highlight is on a directory and the user presses Enter
- **THEN** the browser lists that directory and nothing is attached

### Requirement: The picker is one bordered overlay

The system SHALL draw the picker as a centred bordered block over the transcript, SHALL size it to at least 40 columns and at most 80% of the terminal width, SHALL show at most eight rows with a scroll position, SHALL erase the region when it closes, and SHALL draw no other border, pane, or widget anywhere.

#### Scenario: Narrow terminal
- **WHEN** the terminal is 50 columns
- **THEN** the picker is 40 columns wide and centred

#### Scenario: Long list
- **WHEN** forty profiles are available
- **THEN** eight rows are visible and the highlight scrolls the window

#### Scenario: Close
- **WHEN** the picker closes
- **THEN** no border characters remain on screen

#### Scenario: Transcript untouched
- **WHEN** the picker opens
- **THEN** the rows outside the overlay are unchanged
