## MODIFIED Requirements

### Requirement: The user can start, continue, and pick sessions

The system SHALL start a new session by default, continue the most recent with `-c`, resume a named one with `-r <id>`, resume the most recent with `-r` when no id follows or a flag follows, and open a picker in the interface that lists sessions newest first and starts a new one on request.

#### Scenario: Continue
- **WHEN** the user runs `imp -c`
- **THEN** the newest session is loaded and the input is ready

#### Scenario: Resume without an id
- **WHEN** the user runs `imp -r` or `imp -r --model x`
- **THEN** the newest session is loaded, as with `-c`

#### Scenario: A word is an id
- **WHEN** the user runs `imp -r fix the parser` and no session is named `fix`
- **THEN** imp exits with an error naming `fix` and sends nothing

#### Scenario: Picker
- **WHEN** the user opens the session picker
- **THEN** sessions appear newest first with title, age, and message count, and choosing one loads it

#### Scenario: New from the picker
- **WHEN** the user chooses "new session" in the picker
- **THEN** a fresh session starts and the previous file stays intact

