# Spec Delta

## ADDED Requirements

### Requirement: Always approves a command shape for the session

The system SHALL offer `a` on a command confirmation in `normal` when the command is one simple command, SHALL then run later commands with the same program and first non-flag argument without asking for the rest of that session, SHALL NOT offer it in `strict`, for sudo, for compound commands, or for writes and memory saves, and SHALL forget every always on `/new` and on exit.

#### Scenario: Always npm test
- **WHEN** the user answers `a` to `npm test`
- **THEN** a later `npm test -- --watch` runs without asking and `npm install` still asks

#### Scenario: Program alone
- **WHEN** the user answers `a` to `make`
- **THEN** a later `make` runs without asking and `make clean` still asks

#### Scenario: Flags do not widen it
- **WHEN** the user answers `a` to `rm -rf build`
- **THEN** a later `rm -rf build` runs without asking and `rm -rf src` still asks

#### Scenario: Compound command
- **WHEN** the command contains `&&`, `;`, `|`, or a redirect
- **THEN** the prompt does not offer `a`

#### Scenario: Strict
- **WHEN** the mode is `strict`
- **THEN** no prompt offers `a`

#### Scenario: Deny list first
- **WHEN** a command is on the deny list after an always for its program
- **THEN** it is still refused

#### Scenario: Per session
- **WHEN** the user runs `/new` after approving `npm test` always
- **THEN** `npm test` asks again
