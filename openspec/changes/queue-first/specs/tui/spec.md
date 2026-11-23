## MODIFIED Requirements

### Requirement: Enter steers or queues by a visible mode

The system SHALL keep a per-session send mode, queue by default or as `send_mode` sets it, SHALL toggle it on Ctrl+Q, SHALL show it only as the prompt glyph, a muted `»` for queue and `>` for steer, SHALL on Enter in steer mode, or on Ctrl+S in either mode, give the draft to the running turn before the model's next step and draw it as a user row marked `(steered)`, and SHALL send on Enter or Ctrl+S when no turn runs.

#### Scenario: Queue by default
- **WHEN** a turn is running in a new session and the user types `also add tests` and presses Enter
- **THEN** it shows above the composer, the prompt glyph is `»`, and it is sent when the turn ends

#### Scenario: Steer
- **WHEN** a turn is running a tool and the user types `use pnpm, not npm` and presses Ctrl+S
- **THEN** the model's next request carries it, a user row says `(steered)`, and the mode is still queue

#### Scenario: Steered too late
- **WHEN** the user steers a message while the model writes its final reply
- **THEN** the message starts the next turn as soon as this one ends, or waits at the front of the queue if the user stopped the turn

#### Scenario: Toggle
- **WHEN** a turn is running in queue mode and the user presses Ctrl+Q
- **THEN** the prompt glyph is `>` and the next Enter steers

#### Scenario: Setting
- **WHEN** the config holds `send_mode = steer`
- **THEN** a new session's first Enter during a turn steers

#### Scenario: Idle
- **WHEN** no turn runs
- **THEN** Enter and Ctrl+S both send the message, the glyph is `>`, and the rule carries no label

#### Scenario: Empty draft
- **WHEN** the draft is empty and the user presses Ctrl+S
- **THEN** nothing happens

### Requirement: Queued messages are edited in place

The system SHALL walk from an empty draft through queued messages, newest first, then history on ↑, and back on ↓, SHALL open one with the rule reading `editing n/m · Enter save · ^S steer|send · ^X del`, SHALL save it in place on Enter or on leaving it, SHALL remove it when saved empty, SHALL send it on `Ctrl+S`, SHALL delete it on `Ctrl+X` and leave the next entry open, SHALL report `queue: deleted n`, and SHALL show `↑ edit queue` while messages wait and no turn runs.

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
- **WHEN** a turn runs, a message is queued and open for editing, and the user presses Ctrl+S
- **THEN** the turn receives that text before its next step, a user row says `(steered)`, and the message is gone from the queue

#### Scenario: Send the open message when idle
- **WHEN** no turn runs and the user presses Ctrl+S on an open queued message
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
- **THEN** the rule reads `editing 1/1 · Enter save · ^S steer · ^X del` and is not hidden

#### Scenario: Idle queue hint
- **WHEN** no turn runs, the draft is empty, and messages wait in the queue
- **THEN** the rule reads `↑ edit queue`
