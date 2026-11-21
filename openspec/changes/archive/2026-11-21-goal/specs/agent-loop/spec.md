## ADDED Requirements

### Requirement: A goal runs over several turns

The system SHALL, while a goal is set, start another turn after each turn whose reply's last line is neither `GOAL MET` nor `GOAL BLOCKED`, with a note restating the goal and the round, up to `goal_rounds` rounds (default 5, at most 20), and SHALL end the goal when it is met, blocked, out of rounds, cancelled, interrupted by a user message, refused at a confirmation, or failed with an error.

#### Scenario: Met in two rounds
- **WHEN** the first round's reply ends without a marker and the second ends with `GOAL MET`
- **THEN** exactly two turns run and an info row says the goal was met

#### Scenario: Out of rounds
- **WHEN** `goal_rounds = 2` and no reply carries a marker
- **THEN** two turns run and an info row says the rounds ran out

#### Scenario: Blocked
- **WHEN** a reply ends with `GOAL BLOCKED: needs the API key`
- **THEN** no further round starts and the reason is shown

#### Scenario: Declined
- **WHEN** the user declines a confirmation during a round
- **THEN** no further round starts

#### Scenario: Mentioned, not claimed
- **WHEN** a reply says "I will write GOAL MET when done" in its middle and ends otherwise
- **THEN** another round starts
