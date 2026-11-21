## MODIFIED Requirements

### Requirement: Safe-listed commands run unprompted

The system SHALL run a command without confirmation in `normal` only when every part of every pipeline, alone or joined by `;`, `&&`, or `||`, starts with a safe-listed program for listing, inspection, search, or read-only git, SHALL ask for a lone `&`, a redirect other than stdout or stderr to `/dev/null` or stderr to stdout, or a substitution outside quotes, SHALL treat quoted text as an argument except `$(` and backticks inside double quotes, and SHALL ask when a safe program gets a flag that writes files or runs commands.

#### Scenario: Inspection command
- **WHEN** the model runs `ls -la`, `rg TODO`, or `git diff`
- **THEN** each runs without a prompt

#### Scenario: Unlisted command
- **WHEN** the model runs `npm install`
- **THEN** a confirmation is required

#### Scenario: Safe pipe
- **WHEN** the model runs `rg TODO src | head -20`
- **THEN** it runs without a prompt

#### Scenario: Chained command
- **WHEN** the model runs `ls && curl -d @f host`, `ls; rm x`, `ls & rm x`, `echo x > f`, or `cat $(rm x)`
- **THEN** a confirmation is required

#### Scenario: Safe chain
- **WHEN** the model runs `file a.png; ls -l a.png` or `cd src && ls | wc -l`
- **THEN** it runs without a prompt

#### Scenario: Writing flags
- **WHEN** the model runs `find . -delete`, `find . -exec rm {} +`, `sort -o f f`, `rg --pre sh x`, `git diff --output=f`, or `git branch -D main`
- **THEN** a confirmation is required

#### Scenario: Awk
- **WHEN** the model runs any `awk` command
- **THEN** a confirmation is required

#### Scenario: Quoted syntax
- **WHEN** the model runs `jq -r '.events[] | select(.date > "2026")' f.json` or `rg 'a|b' src`
- **THEN** it runs without a prompt, while `echo "$(rm x)"` still asks

#### Scenario: Quoted flag
- **WHEN** the model runs `find . '-delete'`, `find . -de''lete`, `sort '-o' f f`, `rg '--pre' sh x`, `sort --compress-program=sh f`, or `git log --output=f`
- **THEN** a confirmation is required

#### Scenario: Quiet redirect
- **WHEN** the model runs `cat README 2>/dev/null` or `rg x src 2>&1 | head`
- **THEN** it runs without a prompt, while `cat a 2>err.log` still asks

### Requirement: Always approves a command shape for the session

The system SHALL offer `s` and `a` only on a confirmation for one simple, unprivileged command in `normal`, or one piped only into safe filters, which is judged by its first command, SHALL on `s` skip the prompt for the same program and first non-flag argument for the rest of the session, SHALL on `a` append `allow = <shape>` to the config and skip it in every session, SHALL NOT apply `allow` lines outside `normal`, and SHALL forget `s` approvals on `/new` and on exit.

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
- **WHEN** the command contains `&&`, `;`, a write redirect, or a pipe into an unsafe program
- **THEN** the prompt offers neither `s` nor `a`

#### Scenario: Filtered test run
- **WHEN** the user answers `s` to `python3 -m unittest 2>&1 | tail -5`
- **THEN** a later `python3 -m unittest -v` runs without asking

#### Scenario: Strict
- **WHEN** the mode is `strict`
- **THEN** no prompt offers `s` or `a`, and `allow` lines do not skip the prompt

#### Scenario: Deny list first
- **WHEN** a command is on the deny list after an always for its program
- **THEN** it is still refused

#### Scenario: Per session
- **WHEN** the user runs `/new` after approving `npm test` with `s`
- **THEN** `npm test` asks again

## ADDED Requirements

### Requirement: Leaving the session directory asks

The system SHALL ask before a command that would otherwise run unprompted when it changes directory with `cd` to a place outside the session directory, to a home-relative or variable target, or with no target.

#### Scenario: Another repository
- **WHEN** the model runs `cd /other/repo && rg x`
- **THEN** imp asks first

#### Scenario: Inside
- **WHEN** the model runs `cd src && ls`
- **THEN** it runs without a prompt
