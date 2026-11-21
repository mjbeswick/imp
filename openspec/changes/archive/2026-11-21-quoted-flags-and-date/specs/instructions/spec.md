## MODIFIED Requirements

### Requirement: The prompt carries the date

The system SHALL tell the model today's date in a system note before the first request of each day in every session, and SHALL tell the model in the prompt to search and then open the best source for anything that changes over time, and to say how current the answer is.

#### Scenario: Recent event
- **WHEN** the user asks who won the most recent race of a sport
- **THEN** the model has been told today's date, so its search is not tied to its training year

#### Scenario: Past midnight
- **WHEN** a session started yesterday sends its first request today
- **THEN** a new note gives today's date

#### Scenario: Opened later
- **WHEN** imp has run for days and the user opens a session with `/new`
- **THEN** that session is told today's date, not the day imp started
