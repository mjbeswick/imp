## MODIFIED Requirements

### Requirement: Queued messages are edited in place

The system SHALL walk from an empty draft through queued messages, newest first, and then history on ↑, and back on ↓, SHALL open a queued message in the composer with the rule reading `editing queued n/m · Enter save · ↑↓ move`, SHALL save it in its place on Enter or on leaving it, SHALL remove it when saved empty, SHALL send the open message now on `Ctrl+Q`, as a steer into the running turn or as a new turn when none runs, removing it from the queue, SHALL delete it on `Ctrl+X` and leave the entry that follows open, SHALL close the editor on `Ctrl+C` with the message unchanged, and SHALL release the queue whenever the editor closes after the turn ended.

#### Scenario: Edit the next message
- **WHEN** two messages are queued and the user presses ↑ twice on an empty draft
- **THEN** the older one is open in the composer and highlighted in the queue

#### Scenario: Save
- **WHEN** the user changes the open message and presses Enter
- **THEN** it keeps its place in the queue with the new text

#### Scenario: Delete by emptying
- **WHEN** the user clears the open message and presses Enter
- **THEN** it is gone from the queue

#### Scenario: Steer the open message
- **WHEN** a turn runs, a message is queued and open for editing, and the user presses Ctrl+Q
- **THEN** the turn receives that text before its next step, a user row says `(steered)`, and the message is gone from the queue

#### Scenario: Send the open message when idle
- **WHEN** no turn runs and the user presses Ctrl+Q on an open queued message
- **THEN** that message starts a turn and is gone from the queue

#### Scenario: Delete with a key
- **WHEN** two messages are queued, the first is open, and the user presses Ctrl+X
- **THEN** the first is gone and the second is open for editing

#### Scenario: Close without saving
- **WHEN** the user presses Ctrl+C while a queued message is open
- **THEN** the editor closes, the message stays queued as it was, and imp does not quit

#### Scenario: Leaving after the turn ended
- **WHEN** a turn ends while a queued message is open and the user presses ↑ into history
- **THEN** the oldest queued message starts, with no keypress beyond that ↑
