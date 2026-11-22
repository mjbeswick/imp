## MODIFIED Requirements

### Requirement: The interface is keyboard first

The system SHALL send on Enter, steer or queue during a turn by the mode, newline on Shift+Enter, Alt+Enter, or Ctrl+J, cancel a turn on Esc, retry on Ctrl+R, toggle tool rows on Ctrl+E, toggle thinking on Ctrl+T and Ctrl+H, clear the view on Ctrl+L, scroll on PgUp and PgDn, stop a turn or clear a draft on Ctrl+C, ask for a second Ctrl+C before quitting, exit on Ctrl+D, decode control bytes and `CSI` sequences alike, and never type escape parameters as text.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Clear a draft
- **WHEN** the session is idle, the composer holds text, and the user presses Ctrl+C
- **THEN** the text is deleted, nothing is sent, and the app stays open

#### Scenario: Quit guard
- **WHEN** the composer is empty and the user presses Ctrl+C
- **THEN** the app exits on that press

#### Scenario: Draft mid-turn
- **WHEN** a turn is streaming, the composer holds text, and the user presses Ctrl+C
- **THEN** the draft is cleared and the turn keeps running

#### Scenario: Cancel then exit
- **WHEN** a turn is streaming, the composer is empty, and the user presses Ctrl+C
- **THEN** the app exits on that press and the turn ends with it

#### Scenario: Picker open
- **WHEN** a picker is open, the composer is empty, and the user presses Ctrl+C
- **THEN** the app exits instead of swallowing the key

#### Scenario: Picker open with text
- **WHEN** a picker is open and the composer holds text
- **THEN** Ctrl+C closes the picker, clears the text, and the app stays open

#### Scenario: Confirmation open
- **WHEN** a command confirmation or an `ask` question is open and the user presses Ctrl+C
- **THEN** the app exits on that press

#### Scenario: Start a second line
- **WHEN** the user types a line and presses Shift+Enter
- **THEN** the caret moves to a new row, nothing is sent, and the first line stays in the composer

#### Scenario: Terminals without modifier reporting
- **WHEN** the terminal never reports Shift+Enter separately
- **THEN** Alt+Enter and Ctrl+J still start a new line

#### Scenario: Cancel a running turn
- **WHEN** the user presses Esc while a turn is streaming or running a tool
- **THEN** the turn stops at its next check, the partial answer stays on screen, and the input line accepts text again

#### Scenario: Disambiguated control keys
- **WHEN** the terminal reports Ctrl+C as `CSI 99;5u`
- **THEN** it follows the same clear-or-exit rule as a raw 0x03

#### Scenario: Page keys
- **WHEN** the terminal sends `CSI 5~` or `CSI 6~`
- **THEN** the transcript pages up or down and no `5~` or `6~` text reaches the composer

#### Scenario: Expand moved
- **WHEN** the user presses Ctrl+E after a tool call
- **THEN** the latest turn's tool rows expand, and Ctrl+O goes to the next waiting session instead

#### Scenario: Newline keys
- **WHEN** the user presses Alt+Enter or Ctrl+J, or the terminal reports Shift+Enter
- **THEN** a newline is inserted and nothing is sent

#### Scenario: Ctrl+C stops a turn
- **WHEN** a turn is running, the draft is empty, and the user presses Ctrl+C
- **THEN** the turn stops and imp stays open

#### Scenario: Ctrl+C clears a draft first
- **WHEN** a turn is running and the draft holds text
- **THEN** Ctrl+C clears the draft and the turn goes on

#### Scenario: Ctrl+C on a prompt
- **WHEN** a confirmation is open and the user presses Ctrl+C
- **THEN** the command does not run, the turn stops, and imp stays open

#### Scenario: Quitting takes two
- **WHEN** nothing runs, the draft is empty, and the user presses Ctrl+C
- **THEN** a line says `Ctrl+C again to quit`, and a second Ctrl+C within two seconds exits

### Requirement: Twenty-five slash commands, no more

The system SHALL provide exactly `/help`, `/keys`, `/diff`, `/new`, `/goal`, `/copy`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/mode`, `/next`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/queue`, `/rename`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty-five commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Rename
- **WHEN** the user runs `/rename fix the parser`
- **THEN** the current session carries that title and no request is sent

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

## REMOVED Requirements

### Requirement: Messages can be stashed for the session
**Reason**: The queue holds messages that wait; a queue with nothing running is a stash.
**Migration**: `/queue`, `/queue pop n`, `/queue delete n`, and `/queue clear` replace `/stash` and its forms.

## ADDED Requirements

### Requirement: Messages wait in a queue

The system SHALL add the draft to a session queue on Enter while a turn runs in queue mode, SHALL draw queued messages dim above the composer, SHALL send the oldest when a turn ends unless a queued message is open or the user stopped the turn, SHALL NOT restart the running turn's meter for a queued message, and SHALL add, list, pop, delete, and clear queued messages with `/queue <text>`, `/queue`, `/queue pop n`, `/queue delete n`, and `/queue clear`.

#### Scenario: Queue during a turn
- **WHEN** a turn is running in queue mode and the user sends `also add tests`
- **THEN** it shows above the composer, not in the transcript, and is sent when the turn ends

#### Scenario: A stopped turn
- **WHEN** the user stops a turn with Esc while a message is queued
- **THEN** the message stays queued and no turn starts

#### Scenario: Pop
- **WHEN** the user runs `/queue pop 1`
- **THEN** the first queued message is in the draft and gone from the queue

### Requirement: Queued messages are edited in place

The system SHALL walk from an empty draft through queued messages, newest first, and then history on ↑, and back on ↓, SHALL open a queued message in the composer with the rule reading `editing queued n/m · Enter save · ↑↓ move`, SHALL save it in its place on Enter or on leaving it, and SHALL remove it when saved empty.

#### Scenario: Edit the next message
- **WHEN** two messages are queued and the user presses ↑ twice on an empty draft
- **THEN** the older one is open in the composer and highlighted in the queue

#### Scenario: Save
- **WHEN** the user changes the open message and presses Enter
- **THEN** it keeps its place in the queue with the new text

#### Scenario: Delete by emptying
- **WHEN** the user clears the open message and presses Enter
- **THEN** it is gone from the queue

### Requirement: Enter steers or queues by a visible mode

The system SHALL keep a per-session send mode, steer by default, SHALL toggle it on Ctrl+Q, SHALL show it while a turn runs as a label set into the rule above the composer, `steer · ^Q queue` or `queue · ^Q steer`, and as the prompt glyph, `>` for steer and a muted `»` for queue, SHALL on Enter in steer mode give the draft to the running turn before the model's next step and draw it as a user row marked `(steered)`, and SHALL send on Enter when no turn runs, whatever the mode.

#### Scenario: Steer
- **WHEN** a turn is running a tool in steer mode and the user types `use pnpm, not npm` and presses Enter
- **THEN** the model's next request carries it and the turn goes on

#### Scenario: Steered too late
- **WHEN** the user steers a message while the model writes its final reply
- **THEN** the message starts the next turn as soon as this one ends, or waits at the front of the queue if the user stopped the turn

#### Scenario: Toggle
- **WHEN** a turn is running in steer mode and the user presses Ctrl+Q
- **THEN** the rule reads `queue · ^Q steer`, the prompt glyph is `»`, and the next Enter queues

#### Scenario: Idle
- **WHEN** no turn runs and the mode is queue
- **THEN** Enter sends the message, the glyph is `>`, and the rule carries no label
