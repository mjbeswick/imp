## MODIFIED Requirements

### Requirement: Tab cycles the agent and Shift+Tab the mode

The system SHALL cycle the permission mode through `normal`, `strict`, and `yolo` on Shift+Tab, applying it at once, SHALL cycle agent profiles on Tab when the draft is empty and no picker is open, applying a change made during a turn when that turn ends, SHALL write one transcript line for each mode change, and SHALL show the mode in the status line when it is not `normal`.

#### Scenario: Mode
- **WHEN** the mode is `strict` and the user presses Shift+Tab
- **THEN** the mode is `yolo`, the prompt and rule turn to the error colour, and one line says so

#### Scenario: Agent
- **WHEN** two agents exist, the draft is empty, and the user presses Tab
- **THEN** the other agent is active for the next message

#### Scenario: Mid-turn agent
- **WHEN** a turn is running and the user presses Tab
- **THEN** the prompt names the coming agent and the switch happens when the turn ends

#### Scenario: Text in the draft
- **WHEN** the draft holds `fix` and the user presses Tab
- **THEN** nothing changes

#### Scenario: Agent before the prompt
- **WHEN** more than one agent exists and `plan` is active
- **THEN** the composer's first row starts with `plan >` and no `agent:` line is written on a switch

#### Scenario: One agent
- **WHEN** only one agent exists
- **THEN** the composer starts with `>` alone

### Requirement: Enter steers or queues by a visible mode

The system SHALL keep a per-session send mode, steer by default, SHALL toggle it on Ctrl+Q, SHALL show it only as the prompt glyph, `>` for steer and a muted `»` for queue, SHALL on Enter in steer mode give the draft to the running turn before the model's next step and draw it as a user row marked `(steered)`, and SHALL send on Enter when no turn runs, whatever the mode.

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
