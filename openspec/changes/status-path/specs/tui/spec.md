## MODIFIED Requirements

### Requirement: The status line shows the session directory

The system SHALL draw the session directory at the right end of the status line, SHALL shorten the home directory to `~`, SHALL keep at least two spaces between it and the left figures, SHALL, when the whole path does not fit, shorten it in this order and use the first form that fits: every parent cut to its first character with the last folder whole, then the parents as one `…`, then the last folder cut from the front when at least 8 columns are left, and SHALL omit it rather than cut the model name or figures when even that does not fit.

#### Scenario: Wide terminal
- **WHEN** the terminal is 100 columns and the root is the home directory's `Projects/imp`
- **THEN** the status line ends with `~/Projects/imp` in the last column

#### Scenario: Narrow terminal
- **WHEN** the root is `~/Projects/imp/src/providers` and 6 columns are left after the figures and two spaces
- **THEN** the directory is absent and the model name is whole

#### Scenario: Abbreviated parents
- **WHEN** the root is `~/Projects/imp/src/providers` and 20 columns are left for it
- **THEN** the status line ends with `~/P/i/s/providers`

#### Scenario: Last folder only
- **WHEN** the same root has 12 columns left
- **THEN** the status line ends with `…/providers`

#### Scenario: Dot folder
- **WHEN** the root is `~/.config/imp/agents` and the parents must be cut
- **THEN** `.config` shows as `.c`
