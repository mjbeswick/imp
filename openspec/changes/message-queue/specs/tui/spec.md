## ADDED Requirements

### Requirement: Messages sent during a turn are queued

The system SHALL hold a message sent while a turn runs in a queue drawn dim above the composer, SHALL send queued messages oldest first as each turn ends, SHALL pull the newest back into an empty draft on Up and pause the queue, SHALL resume on the next send, and SHALL NOT restart the running turn's meter for a queued message.

#### Scenario: Queue
- **WHEN** a turn is running and the user sends `also add tests`
- **THEN** it shows above the composer, not in the transcript, and is sent when the turn ends

#### Scenario: Edit a queued message
- **WHEN** two messages are queued and the user presses Up on an empty draft
- **THEN** the newer one is in the draft, the queue shows paused, and a turn ending sends nothing

#### Scenario: Resume
- **WHEN** the user edits the pulled message and presses Enter while a turn runs
- **THEN** it returns to the end of the queue and the queue is no longer paused

#### Scenario: Meter
- **WHEN** a message is queued four steps into a turn
- **THEN** the status line still counts four steps

### Requirement: A draft can steer the running turn

The system SHALL, on Ctrl+X, give the draft to the running turn before the model's next step and draw it as a user row marked `(steered)`, and SHALL send it as Enter does when no turn runs.

#### Scenario: Steer
- **WHEN** a turn is running a tool and the user types `use pnpm, not npm` and presses Ctrl+X
- **THEN** the model's next request carries that message and the turn goes on

### Requirement: Ctrl+S stashes the draft

The system SHALL stash a non-empty draft on Ctrl+S and clear it, and SHALL pop the newest stash into an empty draft.

#### Scenario: Stash and pop
- **WHEN** the user presses Ctrl+S on `half a thought`, then Ctrl+S again on the empty draft
- **THEN** the draft is `half a thought` again and the stash is empty
