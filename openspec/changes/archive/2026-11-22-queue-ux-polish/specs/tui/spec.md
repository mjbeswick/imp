## MODIFIED Requirements

### Requirement: Queued messages are edited in place

The system SHALL walk from an empty draft through queued messages, newest first, then history on ↑, and back on ↓, SHALL open one with the rule reading `editing n/m · Enter save · ^Q steer|send · ^X del`, SHALL save it in place on Enter or on leaving it, SHALL remove it when saved empty, SHALL send it on `Ctrl+Q`, SHALL delete it on `Ctrl+X` and leave the next entry open, SHALL report `queue: deleted n`, and SHALL show `↑ edit queue` while messages wait and no turn runs.

#### Scenario: Edit the next message
- **WHEN** two messages are queued and the user presses ↑ twice on an empty draft
- **THEN** the older one is open in the composer and highlighted in the queue

#### Scenario: Save
- **WHEN** the user changes the open message and presses Enter
- **THEN** it keeps its place in the queue with the new text

#### Scenario: Delete by emptying
- **WHEN** the user clears the open message and presses Enter
- **THEN** it is gone from the queue and `queue: deleted 1` is reported

#### Scenario: Steer the open message
- **WHEN** a turn runs, a message is queued and open for editing, and the user presses Ctrl+Q
- **THEN** the turn receives that text before its next step, a user row says `(steered)`, and the message is gone from the queue

#### Scenario: Send the open message when idle
- **WHEN** no turn runs and the user presses Ctrl+Q on an open queued message
- **THEN** that message starts a turn and is gone from the queue

#### Scenario: Delete with a key
- **WHEN** two messages are queued, the first is open, and the user presses Ctrl+X
- **THEN** the first is gone, `queue: deleted 1` is reported, and the second is open for editing

#### Scenario: Close without saving
- **WHEN** the user presses Ctrl+C while a queued message is open
- **THEN** the editor closes, the message stays queued as it was, and imp does not quit

#### Scenario: Leaving after the turn ended
- **WHEN** a turn ends while a queued message is open and the user presses ↑ into history
- **THEN** the oldest queued message starts, with no keypress beyond that ↑

#### Scenario: Rule names the keys
- **WHEN** a queued message is open while a turn runs at 54 columns
- **THEN** the rule reads `editing 1/1 · Enter save · ^Q steer · ^X del` and is not hidden

#### Scenario: Idle queue hint
- **WHEN** no turn runs, the draft is empty, and messages wait in the queue
- **THEN** the rule reads `↑ edit queue`

### Requirement: Suggestions are accepted with one key

The system SHALL draw a suggestion greyed out in an empty input with no key hint after it, SHALL show `→ accept` in the rule while one is shown, SHALL fill the input with it on Right or Enter, SHALL drop it as soon as the user types, sends, or a turn starts, and SHALL NOT send it on Enter alone.

#### Scenario: Accept
- **WHEN** the input is empty, `run the tests` is suggested, and the user presses Right then Enter
- **THEN** `run the tests` is sent

#### Scenario: Enter fills
- **WHEN** `run the tests` is suggested and the user presses Enter once
- **THEN** the input holds `run the tests` and nothing is sent

#### Scenario: Typed over
- **WHEN** a suggestion is shown and the user types `n`
- **THEN** the input holds `n` and the suggestion is gone

#### Scenario: Late arrival
- **WHEN** a suggestion arrives after the user has started typing
- **THEN** it is not shown

#### Scenario: No hint
- **WHEN** `run the tests` is suggested
- **THEN** the input row shows `run the tests` and no `Tab` after it

#### Scenario: Rule hint
- **WHEN** a suggestion is shown and no turn runs
- **THEN** the rule reads `→ accept`

### Requirement: The status line shows the session directory

The system SHALL draw the session directory at the right end of the status line, SHALL shorten the home directory to `~`, SHALL keep two spaces before it, SHALL shorten a path that does not fit in this order and use the first form that fits: the last two folders whole with earlier parents cut to one character, then the last folder whole with earlier parents as one `…`, then the last folder cut from the front, and SHALL omit it rather than cut the model name or figures.

#### Scenario: Wide terminal
- **WHEN** the terminal is 100 columns and the root is the home directory's `Projects/imp`
- **THEN** the status line ends with `~/Projects/imp` in the last column

#### Scenario: Narrow terminal
- **WHEN** the root is `~/Projects/imp/src/providers` and 6 columns are left after the figures and two spaces
- **THEN** the directory is absent and the model name is whole

#### Scenario: Abbreviated parents
- **WHEN** the root is `~/Projects/imp/src/providers` and 20 columns are left for it
- **THEN** the status line ends with `~/P/i/src/providers`, keeping two folders whole

#### Scenario: Last folder only
- **WHEN** the same root has 12 columns left
- **THEN** the status line ends with `…/providers`

#### Scenario: Dot folder
- **WHEN** the root is `~/.config/imp/agents` and the parents must be cut
- **THEN** `.config` shows as `.c`

### Requirement: Interface rows carry emphasis

The system SHALL set thinking rows and other muted asides in italic, SHALL leave the activity spinner glyph upright while the rest of its row is italic, SHALL paint picker descriptions muted on rows that are not highlighted, SHALL paint the picker title bold, SHALL paint the status line's unseen and waiting counts in the accent colour, and SHALL show `/help` section titles bold and its keys and commands in the accent colour, flush left and whole.

#### Scenario: Thinking
- **WHEN** a thinking row is shown in a colour terminal
- **THEN** it is italic and muted

#### Scenario: Spinner
- **WHEN** the activity spinner is shown while the model works
- **THEN** the glyph is upright and the words after it are italic

#### Scenario: Picker description
- **WHEN** the command picker is open
- **THEN** descriptions on rows that are not highlighted are muted

#### Scenario: Unseen count
- **WHEN** rows arrive below the view
- **THEN** the `↑ n` count is in the accent colour and the rest of the status line stays muted

#### Scenario: Help headings
- **WHEN** the user runs `/help` in a colour terminal
- **THEN** each section title is bold and each key and command is accent

#### Scenario: Help at Hackberry width
- **WHEN** the user runs `/help` at 40 or 54 columns
- **THEN** every row matches the plain help text, flush left and uncut
