# picker Specification

## Purpose
Lets the user choose command values and paths in a centred overlay instead of typing them.

## Requirements

### Requirement: One picker serves command values

The system SHALL provide a single picker with a title, a filter, a scrolling list, a highlighted row, and cancel, SHALL open it for `/agent`, `/sessions`, `/verbosity`, and `/model` with no value, and after a command word and space when that command has values, SHALL apply a typed value directly, SHALL filter case-insensitively, SHALL move with Up/Down and page with PgUp/PgDn, SHALL choose with Enter, and SHALL cancel with Esc unchanged.

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

### Requirement: Picker rows can carry shortcut actions

The system SHALL allow a picker row to declare single-key actions, SHALL show those keys in a footer hint with the row's action names, SHALL run the action against the highlighted row without closing the picker, and SHALL keep typing filtering when the filter is not empty.

#### Scenario: Row shortcut
- **WHEN** the session picker highlights a row and the filter is empty
- **THEN** `r` opens the rename prompt for that row and the picker returns afterwards

#### Scenario: Filter wins
- **WHEN** the filter contains text and the user types `r`
- **THEN** `r` filters the list rather than renaming

#### Scenario: Footer
- **WHEN** a picker has row actions
- **THEN** the footer lists each key and its action next to Enter and Esc

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
