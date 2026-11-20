## MODIFIED Requirements

### Requirement: Always approves a command shape for the session

The system SHALL offer `s` and `a` on a command confirmation in `normal` when the command is one simple command, SHALL on `s` run later commands with the same program and first non-flag argument without asking for the rest of that session, SHALL on `a` append `allow = <shape>` to the config file and run matching commands without asking in every live and later session, SHALL NOT offer either in `strict`, for sudo, for compound commands, or for writes and memory saves, SHALL NOT apply `allow` lines outside `normal`, and SHALL forget every `s` approval on `/new` and on exit.

#### Scenario: Always npm test
- **WHEN** the user answers `s` to `npm test`
- **THEN** a later `npm test -- --watch` runs without asking and `npm install` still asks

#### Scenario: Program alone
- **WHEN** the user answers `s` to `make`
- **THEN** a later `make` runs without asking and `make clean` still asks

#### Scenario: Flags do not widen it
- **WHEN** the user answers `s` to `rm -rf build`
- **THEN** a later `rm -rf build` runs without asking and `rm -rf src` still asks

#### Scenario: Always saved
- **WHEN** the user answers `a` to `npm test`
- **THEN** the config file ends with `allow = npm test`, and `npm test` runs without asking after `/new` and in another live session

#### Scenario: Allow line read
- **WHEN** the config file holds `allow = cargo build` and the mode is `normal`
- **THEN** `cargo build --release` runs without asking

#### Scenario: Compound command
- **WHEN** the command contains `&&`, `;`, `|`, or a redirect
- **THEN** the prompt offers neither `s` nor `a`

#### Scenario: Strict
- **WHEN** the mode is `strict`
- **THEN** no prompt offers `s` or `a`, and `allow` lines do not skip the prompt

#### Scenario: Deny list first
- **WHEN** a command is on the deny list after an always for its program
- **THEN** it is still refused

#### Scenario: Per session
- **WHEN** the user runs `/new` after approving `npm test` with `s`
- **THEN** `npm test` asks again
