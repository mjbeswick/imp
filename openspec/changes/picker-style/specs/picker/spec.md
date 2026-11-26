## REMOVED Requirements

### Requirement: The picker is one bordered overlay

### Requirement: The picker frame carries its labels

## ADDED Requirements

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

The system SHALL give the picker a fixed height of one blank padding row above the title row, the title row, the item rows, and one blank padding row below the item rows, the item rows being three fifths of the terminal rows less one, at least five, at most twenty, and at most the terminal rows less four, SHALL keep that height for any number of items by leaving spare rows blank, and SHALL keep the highlighted row in the window as the height changes.

#### Scenario: Long list
- **WHEN** forty profiles are available in a 30-row terminal
- **THEN** seventeen item rows are visible, the title row shows `1/40` before `esc`, and the highlight scrolls the window

#### Scenario: Short list keeps the height
- **WHEN** the picker has three items in a 30-row terminal
- **THEN** the box is as tall as with forty items and the spare rows are blank

#### Scenario: Resize
- **WHEN** the terminal grows from 30 rows to 40 rows
- **THEN** the box grows to twenty item rows, the upper bound

### Requirement: The picker title row carries its labels

The system SHALL draw the filter in the picker's title row, SHALL show the picker title in muted text as a placeholder while the filter is empty, SHALL show `esc` at the right of the title row with the scroll position before it when the list scrolls, SHALL draw no rule and no key-hint line in the picker, and SHALL paint the selected row across the full inner width.

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

## MODIFIED Requirements

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

### Requirement: Picker rows can carry shortcut actions

The system SHALL allow a picker row to declare single-key actions, SHALL show those keys in the title row before `esc` with the row's action names, SHALL run the action against the highlighted row without closing the picker, and SHALL keep typing filtering when the filter is not empty.

#### Scenario: Row shortcut
- **WHEN** the session picker highlights a row and the filter is empty
- **THEN** `r` opens the rename prompt for that row and the picker returns afterwards

#### Scenario: Filter wins
- **WHEN** the filter contains text and the user types `r`
- **THEN** `r` filters the list rather than renaming

#### Scenario: Footer
- **WHEN** a picker has row actions
- **THEN** the title row lists each key and its action before `esc`

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
