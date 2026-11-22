## MODIFIED Requirements

### Requirement: Allows can be listed and revoked from the mode picker

The system SHALL list, in the `/mode` picker below the modes, every allow for the session, each marked `session` or `always`, SHALL remove the highlighted allow on `d`, deleting an `always` allow's `allow` line from the config and leaving the rest of the file unchanged, SHALL ask again the next time a revoked shape runs, SHALL end the list with an `a` add row that opens `allow = ` and saves the typed shape as an always allow on Enter, SHALL list the shapes asked this session that are not allowed, newest first and at most five, marked `allowed` or `declined`, and SHALL save one on `a` or pre-fill the rule prompt on Enter.

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
- **WHEN** there are no allows and nothing was asked
- **THEN** the picker shows the three modes and the add row

#### Scenario: Add a rule
- **WHEN** the user presses `a` on the add row and types `cargo test`
- **THEN** the config gains `allow = cargo test` and the picker lists it as `always`

#### Scenario: Empty rule
- **WHEN** the user presses Enter on an empty rule prompt
- **THEN** nothing is saved and the prompt stays open

#### Scenario: Recent ask
- **WHEN** `npm test` asked this session and was declined
- **THEN** the picker lists it as `npm test · declined`

#### Scenario: Save an ask
- **WHEN** the user presses `a` on `npm test · declined`
- **THEN** the config gains `allow = npm test` and the next `npm test` runs without asking

#### Scenario: Already allowed
- **WHEN** a shape is allowed and was also asked
- **THEN** it is listed once, as an allow, not among the asks
