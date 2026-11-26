## ADDED Requirements

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

## MODIFIED Requirements

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
