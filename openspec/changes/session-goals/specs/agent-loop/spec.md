## ADDED Requirements

### Requirement: The agent can propose a goal that waits for confirmation

The system SHALL provide a `goal` tool that lets the agent propose a goal for its session, SHALL NOT start the goal's rounds until the user confirms it, SHALL show the goal text in the confirmation, SHALL drop a declined goal and tell the agent the user declined, and SHALL use the round limit from `goal_rounds`.

#### Scenario: Confirmed proposal
- **WHEN** the agent calls the `goal` tool with `set` and the user confirms
- **THEN** the goal starts at round 1 with the proposed text

#### Scenario: Declined proposal
- **WHEN** the user declines the confirmation
- **THEN** no rounds run, the goal is dropped, and the tool result says the user declined

### Requirement: A goal is saved with its session and restores paused

The system SHALL save a session's goal, including its text, round, round limit and paused state, to the session metadata when the goal changes, and SHALL restore a saved running goal as paused when the session is resumed.

#### Scenario: Resume restores paused
- **WHEN** a session with a running goal at round 2 of 5 is resumed
- **THEN** the goal is shown as paused at round 2 of 5 and no round runs until the user resumes it
