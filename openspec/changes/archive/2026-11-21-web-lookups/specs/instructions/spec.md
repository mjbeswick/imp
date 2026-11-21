## ADDED Requirements

### Requirement: The prompt carries the date

The system SHALL state today's date in the composed prompt at session start, and SHALL tell the model to search and then open the best source for anything that changes over time, and to say how current the answer is.

#### Scenario: Recent event
- **WHEN** the user asks who won the most recent race of a sport
- **THEN** the prompt the model received names today's date, so its search is not tied to its training year
