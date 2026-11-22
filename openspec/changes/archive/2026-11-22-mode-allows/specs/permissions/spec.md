## ADDED Requirements

### Requirement: Allows can be listed and revoked from the mode picker

The system SHALL list, in the `/mode` picker below the modes, every allow for the session, each marked `session` or `always`, SHALL remove the highlighted allow on `d`, deleting an `always` allow's `allow` line from the config and leaving the rest of the file unchanged, and SHALL ask again the next time a revoked shape runs.

#### Scenario: Listed
- **WHEN** `npm test` was allowed for the session and `cargo test` always, and the user runs `/mode`
- **THEN** the picker shows the three modes, then `npm test · session` and `cargo test · always`

#### Scenario: Revoke a session allow
- **WHEN** the user highlights `npm test · session` and presses `d`
- **THEN** the row disappears and the next `npm test` asks

#### Scenario: Revoke an always allow
- **WHEN** the user highlights `cargo test · always` and presses `d`
- **THEN** the `allow = cargo test` line is gone from the config, other lines are unchanged, and the next `cargo test` asks

#### Scenario: Mode rows
- **WHEN** the user presses `d` on a mode row
- **THEN** nothing changes

#### Scenario: Nothing allowed
- **WHEN** there are no allows
- **THEN** the picker shows only the three modes
