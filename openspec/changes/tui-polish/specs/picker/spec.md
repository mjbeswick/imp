# Spec Delta

## MODIFIED Requirements

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

The system SHALL align a picker's right column across its rows, SHALL keep the right column whole and cut the left label with `…` when a row does not fit, and SHALL list sessions with the title on the left and the id, age, and message count on the right.

#### Scenario: Long session title
- **WHEN** a session title is wider than the picker allows
- **THEN** the title ends in `…` and its message count is whole

#### Scenario: Aligned counts
- **WHEN** sessions with titles of different lengths are listed
- **THEN** every right column starts in the same column
