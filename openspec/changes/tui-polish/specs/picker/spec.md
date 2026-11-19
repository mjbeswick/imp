# Spec Delta

## MODIFIED Requirements

### Requirement: One picker serves command values

The system SHALL provide a single picker with a title, a filter, a scrolling list, a highlighted row, and cancel, SHALL open it in the chat for `/agent`, `/sessions`, `/verbosity`, `/model`, `/theme`, and `/provider` with no value, and after a command word and space when that command has values, SHALL close it and return the text to the composer when the filter gains a space, SHALL apply a typed value directly, SHALL filter case-insensitively, SHALL move with Up/Down and page with PgUp/PgDn, SHALL choose with Enter, and SHALL cancel with Esc unchanged.

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

### Requirement: The picker is one bordered overlay

The system SHALL draw the picker as a centred block with rounded corners over the transcript, SHALL put its title in the top edge with the scroll position at the right of that edge when the list scrolls, SHALL put its key hints in the bottom edge, SHALL give the filter its own row followed by a separating rule, SHALL size it to at least 40 columns and at most the smaller of 72 columns and 80% of the terminal width, SHALL show at most eight rows, SHALL paint the selected row across the full inner width, SHALL erase the region when it closes, and SHALL draw no other border, pane, or widget anywhere.

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

## ADDED Requirements

### Requirement: Picker columns stay aligned

The system SHALL align a picker's right column across its rows, SHALL keep the right column whole and cut the left label with `…` when a row does not fit, and SHALL list sessions with id, age, and title on the left and the message count on the right.

#### Scenario: Long session title
- **WHEN** a session title is wider than the picker allows
- **THEN** the title ends in `…` and its message count is whole

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
