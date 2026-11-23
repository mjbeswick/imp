## ADDED Requirements

### Requirement: A question leaves the queue alone

The system SHALL leave queued messages untouched while a question or confirmation is open, SHALL NOT offer or use them as its answer, SHALL NOT show `↑ edit queue` while one is open, SHALL end the question's hint row with `· n queued` while messages wait, and SHALL keep them queued when Esc stops the turn.

#### Scenario: Hint
- **WHEN** two messages are queued and the model asks a question
- **THEN** the hint row reads `type to answer   Esc stop · 2 queued` and the rule does not say `↑ edit queue`

#### Scenario: Not an answer
- **WHEN** `use pnpm` is queued, a question is open, and the user presses Enter on an empty answer
- **THEN** nothing is sent and the queue is unchanged

#### Scenario: Esc
- **WHEN** the user presses Esc on the question
- **THEN** the turn stops, both messages stay queued, and no turn starts

#### Scenario: After the answer
- **WHEN** the user answers and the turn goes on and ends
- **THEN** the oldest queued message is sent, as usual
