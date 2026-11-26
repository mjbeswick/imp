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

### Requirement: Picker rows can carry shortcut actions

The system SHALL allow a picker row to declare Ctrl-key actions, SHALL show each key and its action in the footer bar of the picker, SHALL run the action against the highlighted row without closing the picker, and SHALL keep every letter typing into the filter when the filter is not empty.

#### Scenario: Row shortcut
- **WHEN** the session picker highlights a row
- **THEN** Ctrl+R opens the rename prompt for that row and the picker returns afterwards

#### Scenario: Filter wins
- **WHEN** the user types `r` in the session picker
- **THEN** `r` filters the list rather than renaming

#### Scenario: Open picker takes Ctrl+D
- **WHEN** a picker is open and the user presses Ctrl+D
- **THEN** the highlighted row's delete action runs and the application does not quit

#### Scenario: Footer
- **WHEN** a picker has row actions
- **THEN** the footer bar lists each key and its action

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

The system SHALL align a picker's right column across its rows, SHALL keep the right column whole and cut the left label with `…` when a row does not fit, except in the command picker, where the command name stays whole and its description is cut, and SHALL list sessions with their state mark and title on the left and the highlighted session's directory on the right.

#### Scenario: Long session title
- **WHEN** a session title is wider than the picker allows
- **THEN** the title ends in `…` and the highlighted directory is whole

#### Scenario: Command names stay whole
- **WHEN** the command picker is too narrow for a description
- **THEN** the command name is whole and the description is cut

#### Scenario: Aligned counts
- **WHEN** sessions with titles of different lengths are listed
- **THEN** every right column starts in the same column

#### Scenario: Directory on the highlight
- **WHEN** the session picker highlights a session
- **THEN** its directory is shown in the right column, cut from the left, and other rows show no right column

### Requirement: A cut description continues on the highlighted row

The system SHALL continue the highlighted row's description on one further line indented to the description column when it does not fit, SHALL wrap that line at spaces, SHALL cut it with `…` when two lines are still not enough, and SHALL show that line only when the fixed box has a row for it, taking the row from the item at the bottom of the window.

#### Scenario: Description continues
- **WHEN** the command picker highlights `/goal` at 60 columns in a terminal tall enough for the box
- **THEN** its description continues on a second line under the description column and is not cut

#### Scenario: Other rows stay tight
- **WHEN** a command picker row is not highlighted and its description does not fit
- **THEN** that row keeps one line and its description ends in `…`

#### Scenario: Too long for two lines
- **WHEN** the highlighted description does not fit in two lines at 40 columns
- **THEN** the second line ends in `…`

#### Scenario: Short screen
- **WHEN** the fixed box has no row left for the second line
- **THEN** the highlighted description is cut on one line and every listed item keeps its row

### Requirement: Pickers mark the current value one way

The system SHALL mark the current entry of any picker with a `■` in a left gutter column and with `current` in its right column, SHALL mark the highlighted row with a left accent bar across the full width, SHALL show each agent profile's description in the right column, and SHALL open with the current entry highlighted.

#### Scenario: Model picker
- **WHEN** the model picker opens
- **THEN** the active model's row shows `■` in the gutter, its right column reads `current`, and its label has no `(current)` suffix

#### Scenario: Agent descriptions
- **WHEN** the agent picker opens
- **THEN** each profile shows its description in the right column

#### Scenario: Highlight moves away
- **WHEN** the highlight moves to another row
- **THEN** the current row keeps its `■` and the highlighted row shows the accent bar

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

The system SHALL begin every `/cd` listing with a `./` row for the folder being browsed, followed by a `../` row when a parent exists, SHALL make either the session root on Enter, SHALL open the parent when Tab or Right is pressed on `../`, SHALL ignore Tab and Right on `./`, and SHALL show `esc` at the right of its title row with no other key hint.

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

### Requirement: The session picker searches session text

The system SHALL, when the `/sessions` filter matches no title and the user presses Enter, search the message text of every stored session case-insensitively, SHALL list up to twenty matching sessions newest first with a snippet around the first hit, SHALL title the picker `search "<text>"`, and SHALL resume a session chosen from the results.

#### Scenario: Hit
- **WHEN** a session mentions `wttr.in`, no title contains it, and the user types `wttr` and presses Enter
- **THEN** the picker lists that session with a snippet containing `wttr.in`

#### Scenario: Case
- **WHEN** the user searches `WTTR`
- **THEN** the same session is listed

#### Scenario: No hit
- **WHEN** no session mentions the text
- **THEN** the picker shows the no-match row and the session is unchanged

#### Scenario: Resume from a hit
- **WHEN** the user presses Enter on a result
- **THEN** that session opens as it does from the session list

### Requirement: Working sessions spin in the session picker

The system SHALL mark each live session whose agent is working with a spinner before its message count in `/sessions`, SHALL animate it while the picker is open, and SHALL show no spinner on idle or stored sessions.

#### Scenario: Working
- **WHEN** a live session is streaming a reply and the user opens `/sessions`
- **THEN** its row's right column starts with a spinner frame and the frame advances

#### Scenario: Idle
- **WHEN** a live session is idle
- **THEN** its row shows only the message count

### Requirement: The picker is one borderless overlay

The system SHALL draw the picker as a centred block over the transcript with no border, corner, side wall, or bottom edge, SHALL size it to at least 40 columns and at most the smaller of 72 columns and 80% of the terminal width, SHALL erase the region when it closes, and SHALL draw no border characters anywhere.

#### Scenario: Narrow terminal
- **WHEN** the terminal is 50 columns
- **THEN** the picker is 40 columns wide and centred

#### Scenario: Wide terminal
- **WHEN** the terminal is 200 columns
- **THEN** the picker is 72 columns wide and centred

#### Scenario: No border
- **WHEN** the command picker opens
- **THEN** no border characters appear and the title row reads `commands` at its left with `esc` at its right

#### Scenario: Close
- **WHEN** the picker closes
- **THEN** no picker characters remain on screen

#### Scenario: Transcript untouched
- **WHEN** the picker opens
- **THEN** the rows outside the overlay are unchanged

### Requirement: The picker height follows the terminal

The system SHALL give the picker a fixed height, set by the terminal: the item rows are three fifths of the terminal rows less one, at least five, at most twenty, and at most the terminal rows less five, SHALL keep that height for any number of items by leaving spare rows blank, and SHALL keep the highlighted row in the window as the height changes.

#### Scenario: Long list
- **WHEN** forty profiles are available in a 30-row terminal
- **THEN** seventeen item rows are visible, the title row shows `1/40` before `esc`, and the highlight scrolls the window

#### Scenario: Short list keeps the height
- **WHEN** the picker has three items in a 30-row terminal
- **THEN** the box is as tall as with forty items and the spare rows are blank

#### Scenario: Resize
- **WHEN** the terminal grows from 30 rows to 40 rows
- **THEN** the box grows to twenty item rows, the upper bound

### Requirement: The picker has padding and a gap

The system SHALL draw one blank padding row above the title row, one blank row between the title row and the first item, and one blank padding row below the last item row, SHALL fill the box to its full width with no right margin, and SHALL paint the padding with the panel background.

#### Scenario: Padding and gap
- **WHEN** the picker opens with forty items in a 30-row terminal
- **THEN** the title row has a blank row above it and a blank row below it, the first item follows a blank row, and the item rows reach the box's right edge

### Requirement: The picker title row carries its labels

The system SHALL draw the filter in the picker's title row, SHALL show the picker title in muted text as a placeholder while the filter is empty, SHALL show `esc` at the right of the title row with the scroll position before it when the list scrolls, SHALL draw no rule in the picker and keep key hints in the modal footer bar rather than the title row, SHALL leave one blank column to the right of `esc` in the title row, and SHALL paint the selected row across the full inner width.

#### Scenario: Esc margin
- **WHEN** the picker title row is drawn
- **THEN** `esc` ends one column before the box's right edge, and the item rows still reach that edge

#### Scenario: Placeholder
- **WHEN** the theme picker opens with an empty filter
- **THEN** the title row shows `theme` in muted text with `esc` at its right and no rule below it

#### Scenario: Typed filter
- **WHEN** the user types `dark`
- **THEN** the title row shows `dark` in place of the placeholder and `esc` stays at its right

#### Scenario: Selected row
- **WHEN** a row is highlighted
- **THEN** the highlight spans the full inner width of the picker

### Requirement: The picker is drawn on a panel background

The system SHALL fill the picker's block with a panel background derived from the active theme's text colour, SHALL draw a near-white panel in a light theme and a near-black panel in a dark theme, SHALL keep the panel behind the title row and every item row, and SHALL draw no panel in the mono theme.

#### Scenario: Light theme
- **WHEN** the picker opens in a light theme
- **THEN** its block is filled with a near-white panel and the dark text stays readable on it

#### Scenario: Dark theme
- **WHEN** the picker opens in a dark theme
- **THEN** its block is filled with a near-black panel and the light text stays readable on it

#### Scenario: Highlight on the panel
- **WHEN** a row is highlighted
- **THEN** the highlight takes its own background and the panel resumes after the row

#### Scenario: Mono
- **WHEN** the mono theme is active
- **THEN** the picker has no fill and draws as before

### Requirement: The modal footer shows shortcut keys

The system SHALL draw a footer bar at the bottom of a picker box when the picker declares row actions, SHALL fill the bar with the panel background, SHALL list each action as its name and its key with the key muted, SHALL place the first action at the left and the others at the right in declaration order, and SHALL draw no key-hint text elsewhere in the picker.

#### Scenario: Sessions footer
- **WHEN** the session picker is open
- **THEN** the footer bar lists `current directory ctrl+a` at the left and `new ctrl+n`, `delete ctrl+d`, and `rename ctrl+r` at the right

#### Scenario: No actions
- **WHEN** a picker declares no row actions
- **THEN** no footer bar is drawn and the box keeps its height

### Requirement: The session picker groups sessions by day

The system SHALL group the session picker's rows under a heading for the local calendar day of each session's last activity, SHALL title the first group `Today` and every other group its full date, SHALL draw each heading as a row that is not selectable, SHALL count headings within the fixed window, and SHALL skip headings when the highlight moves.

#### Scenario: Today group
- **WHEN** a session was active today
- **THEN** it appears under `Today`

#### Scenario: Earlier day
- **WHEN** a session was last active on Wednesday 25 November 2026 and it is not today
- **THEN** it appears under the heading `Wed Nov 25 2026`

#### Scenario: Highlight skips a heading
- **WHEN** the highlight moves down from the last session under `Today`
- **THEN** the highlight lands on the first session under the next heading, not on the heading

#### Scenario: Heading in the window
- **WHEN** the window holds a heading and its sessions
- **THEN** the heading takes one row of the window and the box height does not change

### Requirement: Ctrl+A filters sessions to the current directory

The system SHALL filter the session picker to sessions whose directory is the current session root when the user presses Ctrl+A, SHALL clear that filter when the user presses Ctrl+A again, and SHALL leave the typed filter unchanged.

#### Scenario: Filter on
- **WHEN** the user presses Ctrl+A in the session picker
- **THEN** only sessions in the current directory remain listed

#### Scenario: Filter off
- **WHEN** the directory filter is on and the user presses Ctrl+A again
- **THEN** all sessions are listed again
