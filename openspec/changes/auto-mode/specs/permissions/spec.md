## MODIFIED Requirements

### Requirement: One setting selects the policy

The system SHALL read a single `permissions` setting with values `normal`, `strict`, `auto`, or `yolo`, defaulting to `normal`, and SHALL report the active mode at startup.

#### Scenario: Default mode
- **WHEN** the config has no permissions setting
- **THEN** the session starts in `normal` and says so

#### Scenario: Unknown value
- **WHEN** the setting is not one of the four values
- **THEN** the session refuses to start and names the valid values

### Requirement: Everything else asks before acting

The system SHALL require confirmation for any file mutation outside the session directory and for any command not on the safe list, except as `auto` and `yolo` allow, SHALL treat a refusal as a tool result rather than an error, and SHALL let the model continue.

#### Scenario: Approved mutation
- **WHEN** the model writes outside the session directory and the user approves
- **THEN** the write happens and the result is normal

#### Scenario: Refused mutation
- **WHEN** the user refuses
- **THEN** nothing changes and the model receives a result saying the user declined

#### Scenario: Strict mode
- **WHEN** the mode is `strict`
- **THEN** every file mutation and every command prompts, including safe-listed ones

### Requirement: Allows can be listed and revoked from the mode picker

The system SHALL list every allow below the modes in the `/mode` picker, marked `session` or `always`, SHALL remove the highlighted allow on `d`, deleting its `allow` line and leaving the rest alone, SHALL ask again when a revoked shape runs, SHALL end the list with an `a` add row that saves a typed shape as an always allow, and SHALL list the shapes asked this session that are not allowed, newest first and at most five, marked `allowed` or `declined`, saving one on `a`.

#### Scenario: Listed
- **WHEN** `npm test` was allowed for the session and `cargo test` always, and the user runs `/mode`
- **THEN** the picker shows the four modes, then `npm test · session` and `cargo test · always`

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
- **THEN** the picker shows the four modes and the add row

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

## ADDED Requirements

### Requirement: Auto runs contained commands

The system SHALL provide an `auto` mode between `normal` and `yolo`, SHALL run without asking a command line made only of safe-listed commands and development commands from a built-in list, SHALL ask for every other command, and SHALL keep the deny list, the path jail, `allow_sudo`, the secret rules, and the file and `fetch` prompts exactly as in `normal`.

#### Scenario: Development loop
- **WHEN** the mode is `auto` and the model runs `cargo test 2>&1 | tail -5`
- **THEN** it runs with no prompt

#### Scenario: Installs ask
- **WHEN** the mode is `auto` and the model runs `npm install left-pad`
- **THEN** the prompt asks, as in `normal`

#### Scenario: Chains ask
- **WHEN** the mode is `auto` and the model runs `cargo build && ./target/debug/tool`
- **THEN** the prompt asks

#### Scenario: Pushes ask
- **WHEN** the mode is `auto` and the model runs `git push`
- **THEN** the prompt asks

#### Scenario: Deny list
- **WHEN** the mode is `auto` and the model runs `git push --force`
- **THEN** it is refused without a prompt
