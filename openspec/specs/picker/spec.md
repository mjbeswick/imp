# picker Specification

## Purpose
Lets the user choose command values and paths in a centred overlay instead of typing them.

## Requirements

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
- **THEN** the picker lists `low` and `high` and choosing one applies it

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

### Requirement: Image picker attaches a file

The system SHALL open a browser for `/image` with no argument, SHALL show directories and PNG, JPEG, GIF, and WebP files under the session root, SHALL mark directories as navigable, SHALL open the highlighted directory on Enter, Tab, or Right, SHALL attach the highlighted image on Enter, and SHALL leave the browser open when a key opens a directory.

#### Scenario: Browse and attach
- **WHEN** the user runs `/image`, opens `Pictures`, highlights `shot.png`, and presses Enter
- **THEN** the picker closes and the image is attached to the next message

#### Scenario: Non-image file
- **WHEN** a `.txt` file is present
- **THEN** it is not listed

#### Scenario: Directory
- **WHEN** the highlight is on a directory and the user presses Enter
- **THEN** the browser lists that directory and nothing is attached

#### Scenario: Open with an arrow
- **WHEN** the highlight is on a directory and the user presses Right or Tab
- **THEN** the browser lists that directory and nothing is attached

### Requirement: The picker is one bordered overlay

The system SHALL draw the picker as a centred block with rounded corners over the transcript, SHALL size it to at least 40 columns and at most the smaller of 72 columns and 80% of the terminal width, SHALL show at most eight rows, SHALL erase the region when it closes, and SHALL draw no other border, pane, or widget anywhere.

#### Scenario: Narrow terminal
- **WHEN** the terminal is 50 columns
- **THEN** the picker is 40 columns wide and centred

#### Scenario: Wide terminal
- **WHEN** the terminal is 200 columns
- **THEN** the picker is 72 columns wide and centred

#### Scenario: Edges
- **WHEN** the command picker opens
- **THEN** the top edge starts `╭─ commands`, the bottom edge holds `Enter choose · Esc close`, and the corners are `╭╮╰╯`

#### Scenario: Long list
- **WHEN** forty profiles are available
- **THEN** eight rows are visible, the top edge shows the position such as `1/40`, and the highlight scrolls the window

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

### Requirement: Picker columns stay aligned

The system SHALL align a picker's right column across its rows, SHALL keep the right column whole and cut the left label with `…` when a row does not fit, except in the command picker, where the command name stays whole and its description is cut, and SHALL list sessions with id, age, and title on the left and the message count on the right.

#### Scenario: Long session title
- **WHEN** a session title is wider than the picker allows
- **THEN** the title ends in `…` and its message count is whole

#### Scenario: Command names stay whole
- **WHEN** the command picker is too narrow for a description
- **THEN** the command name is whole and the description is cut

#### Scenario: Aligned counts
- **WHEN** sessions with titles of different lengths are listed
- **THEN** every right column starts in the same column

### Requirement: Pickers mark the current value one way

The system SHALL mark the current entry of any picker with `current` in its right column and with no other marker, SHALL show each agent profile's description in the right column, and SHALL open with the current entry highlighted.

#### Scenario: Model picker
- **WHEN** the model picker opens
- **THEN** the active model's right column reads `current` and its label has no `(current)` suffix

#### Scenario: Agent descriptions
- **WHEN** the agent picker opens
- **THEN** each profile shows its description on the right

### Requirement: Browsing pickers show their directory

The system SHALL put the directory being browsed in the title of the `/cd` and `/image` pickers, shortening the home directory to `~` and cutting from the left when it does not fit.

#### Scenario: Browse
- **WHEN** the user opens `/cd` in the home directory's `Projects/imp` and opens `src/`
- **THEN** the title reads `cd ~/Projects/imp/src`

### Requirement: A space in a value filter returns the line

The system SHALL close a value picker and return `/{word} {filter}` to the composer when its filter gains a space, so free-text subcommands can be typed through.

#### Scenario: Provider add
- **WHEN** the user types `/provider add work`
- **THEN** no picker is open and the composer holds the full line

### Requirement: The picker frame carries its labels

The system SHALL put the picker title in the top edge with the scroll position at its right when the list scrolls, SHALL put the key hints in the bottom edge, SHALL give the filter its own row followed by a separating rule, and SHALL paint the selected row across the full inner width.

#### Scenario: Edges
- **WHEN** forty profiles are listed
- **THEN** the top edge starts `╭─ agent`, ends `1/40 ─╮`, and the bottom edge holds the key hints

### Requirement: The directory picker chooses with Enter

The system SHALL open a directory browser for `/cd` with no argument at the session root, SHALL make the highlighted folder the session root on Enter and close, SHALL open the highlighted folder on Tab or Right, SHALL go to the parent on Left, up to the filesystem root, and SHALL change the root only after the choice succeeds.

#### Scenario: Enter chooses
- **WHEN** the highlight is on `src/` and the user presses Enter
- **THEN** the picker closes and the session root becomes the absolute path of `src/`

#### Scenario: Tab opens
- **WHEN** the highlight is on `src/` and the user presses Tab or Right
- **THEN** the browser lists `src/` and the session root is unchanged

#### Scenario: Parent
- **WHEN** the user presses Left inside `src/`
- **THEN** the browser returns to the directory above it

#### Scenario: Filesystem root
- **WHEN** the browser is at `/` and the user presses Left
- **THEN** nothing moves

#### Scenario: Filter and choose
- **WHEN** the user types `pro` and presses Enter
- **THEN** the first matching folder becomes the session root

#### Scenario: Typed path
- **WHEN** the user runs `/cd ~/Projects/imp`
- **THEN** the root changes directly and no browser opens

### Requirement: The directory list starts with this folder and its parent

The system SHALL begin every `/cd` listing with a `./` row for the folder being browsed, followed by a `../` row when a parent exists, SHALL make either the session root on Enter, SHALL open the parent when Tab or Right is pressed on `../`, SHALL ignore Tab and Right on `./`, and SHALL show the keys as `Enter choose · Tab/→ open · ← up · Esc close`.

#### Scenario: Row order
- **WHEN** the user opens `/cd` in a folder with subfolders
- **THEN** the rows read `./`, `../`, then the subfolders

#### Scenario: Leaf folder
- **WHEN** the user opens a folder with no subfolders and presses Enter on `./`
- **THEN** that folder becomes the session root

#### Scenario: Filesystem root
- **WHEN** the browser lists `/`
- **THEN** `./` is the only fixed row and no `../` appears

#### Scenario: Image browser
- **WHEN** the user opens `/image`
- **THEN** neither `./` nor `../` appears
