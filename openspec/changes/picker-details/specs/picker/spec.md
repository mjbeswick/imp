## MODIFIED Requirements

### Requirement: Picker columns stay aligned

The system SHALL align a picker's right column across its rows, SHALL keep the right column whole and cut the left label with `…` when a row does not fit, except in the command picker, where the command name stays whole and its description is cut, SHALL continue the highlighted row's description on one further line indented to the description column when it does not fit, SHALL wrap that line at spaces and cut it with `…` when two lines are still not enough, SHALL grow the box by that line only when the result fits the screen, and SHALL list sessions with id, age, and title on the left and the message count on the right.

#### Scenario: Long session title
- **WHEN** a session title is wider than the picker allows
- **THEN** the title ends in `…` and its message count is whole

#### Scenario: Command names stay whole
- **WHEN** the command picker is too narrow for a description
- **THEN** the command name is whole and the description is cut

#### Scenario: Aligned counts
- **WHEN** sessions with titles of different lengths are listed
- **THEN** every right column starts in the same column

#### Scenario: Description continues
- **WHEN** the command picker highlights `/goal` at 60 columns
- **THEN** its description continues on a second line under the description column and is not cut

#### Scenario: Other rows stay tight
- **WHEN** a command picker row is not highlighted and its description does not fit
- **THEN** that row keeps one line and its description ends in `…`

#### Scenario: Too long for two lines
- **WHEN** the highlighted description does not fit in two lines at 40 columns
- **THEN** the second line ends in `…`

#### Scenario: Short screen
- **WHEN** the picker plus the second line would not fit on screen
- **THEN** the highlighted description is cut on one line and every listed item keeps its row
