## MODIFIED Requirements

### Requirement: The interface is keyboard first

The system SHALL provide Enter to send, or mid-turn to queue, Ctrl+S to steer, Shift+Enter, Alt+Enter, or Ctrl+J for a newline, Esc to cancel a turn, Ctrl+R to retry, Ctrl+T to toggle thinking, Ctrl+E to expand tool rows, Ctrl+L to clear the view, PgUp/PgDn to scroll, Ctrl+C to decline, close, clear, or stop before it quits, and Ctrl+D to exit from an empty draft, SHALL decode those keys from control bytes or `CSI` sequences, and SHALL NOT type escape parameters as text.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Clear a draft
- **WHEN** the session is idle, the composer holds text, and the user presses Ctrl+C
- **THEN** the text is deleted, nothing is sent, and the app stays open

#### Scenario: Quit guard
- **WHEN** the composer is empty, nothing runs, and the user presses Ctrl+C once
- **THEN** the app stays open and asks for a second press

#### Scenario: Draft mid-turn
- **WHEN** a turn is streaming, the composer holds text, and the user presses Ctrl+C
- **THEN** the draft is cleared and the turn keeps running

#### Scenario: Cancel then exit
- **WHEN** a turn is streaming, the composer is empty, and the user presses Ctrl+C twice
- **THEN** the first press stops the turn and the second asks before exiting

#### Scenario: Picker open
- **WHEN** a picker is open, the composer is empty, and the user presses Ctrl+C
- **THEN** the picker closes and the app stays open

#### Scenario: Picker open with text
- **WHEN** a picker is open and the composer holds text
- **THEN** Ctrl+C closes the picker, clears the text, and the app stays open

#### Scenario: Confirmation open
- **WHEN** a command confirmation or an `ask` question is open and the user presses Ctrl+C
- **THEN** it is declined and the app stays open

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
- **THEN** it follows the same rule as a raw 0x03

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

#### Scenario: Ctrl+D keeps a draft
- **WHEN** the composer holds text and the user presses Ctrl+D
- **THEN** nothing happens and the draft stays

#### Scenario: Ctrl+D quits
- **WHEN** the composer is empty and the user presses Ctrl+D
- **THEN** the app exits

#### Scenario: Retry key
- **WHEN** the last reply is done and the user presses Ctrl+R
- **THEN** the reply and its tool rows are replaced by a new turn for the same message

#### Scenario: Thinking key
- **WHEN** the user presses Ctrl+T
- **THEN** thinking switches for the next turn, as `/think` does

### Requirement: Enter steers or queues by a visible mode

The system SHALL queue the draft on Enter while a turn runs, SHALL steer on Ctrl+S, giving the draft to the running turn before the model's next step and drawing it as a user row marked `(steered)`, SHALL show a muted `»` as the prompt glyph while a turn runs and `>` otherwise, SHALL send on Enter or Ctrl+S when no turn runs, and SHALL have no send mode to switch.

#### Scenario: Queue
- **WHEN** a turn is running and the user types `also add tests` and presses Enter
- **THEN** it shows above the composer, the prompt glyph is `»`, and it is sent when the turn ends

#### Scenario: Steer
- **WHEN** a turn is running a tool and the user types `use pnpm, not npm` and presses Ctrl+S
- **THEN** the model's next request carries it, a user row says `(steered)`, and the turn goes on

#### Scenario: Steered too late
- **WHEN** the user steers a message while the model writes its final reply
- **THEN** the message starts the next turn as soon as this one ends, or waits at the front of the queue if the user stopped the turn

#### Scenario: Toggle
- **WHEN** a turn is running and the user presses Ctrl+Q
- **THEN** nothing happens: there is no send mode to switch

#### Scenario: Idle
- **WHEN** no turn runs
- **THEN** Enter and Ctrl+S both send the message, the glyph is `>`, and the rule carries no label

#### Scenario: Empty draft
- **WHEN** the draft is empty and the user presses Ctrl+S
- **THEN** nothing happens

### Requirement: Messages wait in a queue

The system SHALL add the draft to a session queue on Enter while a turn runs, SHALL draw queued messages dim above the composer, SHALL send the oldest when a turn ends unless a queued message is open or the user stopped the turn, SHALL NOT restart the running turn's meter for a queued message, and SHALL add, list, pop, delete, and clear queued messages with `/queue <text>`, `/queue`, `/queue pop n`, `/queue delete n`, and `/queue clear`.

#### Scenario: Queue during a turn
- **WHEN** a turn is running and the user sends `also add tests`
- **THEN** it shows above the composer, not in the transcript, and is sent when the turn ends

#### Scenario: A stopped turn
- **WHEN** the user stops a turn with Esc while a message is queued
- **THEN** the message stays queued and no turn starts

#### Scenario: Pop
- **WHEN** the user runs `/queue pop 1`
- **THEN** the first queued message is in the draft and gone from the queue

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

### Requirement: Ctrl+P opens a palette

The system SHALL open a filterable palette on `Ctrl+P` listing every slash command and the keyboard actions, Ctrl+S and Ctrl+V included, that do something without one, SHALL show each action's shortcut and a command's key when it does the same, SHALL run the highlighted row on Enter, SHALL say `one session` when next session has nowhere to go, SHALL close on Esc, and SHALL keep `/` opening the command list alone.

#### Scenario: Open
- **WHEN** the user presses Ctrl+P with an empty draft
- **THEN** a picker titled `palette` lists the commands and the key actions

#### Scenario: Command
- **WHEN** the user filters to `theme` and presses Enter
- **THEN** the theme picker opens, as `/theme` does

#### Scenario: Keyboard action
- **WHEN** the user picks `expand tools` and presses Enter
- **THEN** the latest turn's tool rows expand, as Ctrl+E does

#### Scenario: Filter
- **WHEN** the user types `copy`
- **THEN** the rows narrow to the copy command and any action whose label holds it

#### Scenario: Close
- **WHEN** the user presses Esc
- **THEN** the palette closes and nothing runs

#### Scenario: Sessions
- **WHEN** two live sessions exist and the user presses Ctrl+N
- **THEN** the next session comes into view, and Ctrl+P still opens the palette

#### Scenario: Keys card
- **WHEN** the user opens the keys card
- **THEN** it lists Ctrl+P for the palette and Ctrl+N for the next session

#### Scenario: Command key
- **WHEN** the palette is open
- **THEN** the `/keys` row ends with `· ?` and the `/next` row ends with `· Ctrl+O`

#### Scenario: Keys without a command
- **WHEN** the user picks `steer now` with text in the draft during a turn
- **THEN** the draft is steered, as Ctrl+S does

#### Scenario: One session
- **WHEN** only one session is live and the user picks `next session`
- **THEN** a line says `one session` and the view stays
