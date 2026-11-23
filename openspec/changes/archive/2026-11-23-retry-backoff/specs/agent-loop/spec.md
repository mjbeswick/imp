## ADDED Requirements

### Requirement: Failed requests back off

The system SHALL retry a request that failed with a rate limit, a server error, a timeout, a dropped connection, or an empty stream up to five times, waiting 1, 2, 4, 8, and 16 seconds plus up to 250 ms of jitter, or the provider's `Retry-After` capped at 60 seconds, SHALL show the wait as `retrying in Ns · n/5 · Esc stop` in the status line, SHALL stop at once on Esc, SHALL fail at once on any other error, and SHALL end the error row of a failed turn with `Ctrl+R retry`.

#### Scenario: Rate limit
- **WHEN** the provider answers 429 twice and then succeeds
- **THEN** the turn completes after two waits and no error row is shown

#### Scenario: Retry-After
- **WHEN** a 429 carries `Retry-After: 3`
- **THEN** the next attempt starts after three seconds

#### Scenario: Not retryable
- **WHEN** the provider answers 401
- **THEN** no retry is made and the error row ends with `Ctrl+R retry`

#### Scenario: Stopped while waiting
- **WHEN** the user presses Esc during a wait
- **THEN** the wait ends at once and no further request is sent
