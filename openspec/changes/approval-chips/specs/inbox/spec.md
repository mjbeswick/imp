## MODIFIED Requirements

### Requirement: Act from the inbox

The system SHALL open the highlighted session with Enter, SHALL answer its pending confirmation with `y`, `n`, `s`, or `a` without opening it, following the same rules as the prompt, SHALL close an idle session with `x`, and SHALL show `stop it first` when `x` is pressed on a working or waiting session.

#### Scenario: Approve in place
- **WHEN** a row shows `needs approval` for `run npm test` and the user presses `y`
- **THEN** the command runs in that session, the inbox stays open, and the row becomes `working`

#### Scenario: Approve for the session
- **WHEN** a row shows `needs approval` for `run npm test` and the user presses `s`
- **THEN** the command runs and later `npm test` runs in that session without asking

#### Scenario: Open a session
- **WHEN** the user presses Enter on a row
- **THEN** the inbox closes and that session is in view

#### Scenario: Close a session
- **WHEN** the user presses `x` on an idle row
- **THEN** the session leaves the inbox and stays in `/sessions`
