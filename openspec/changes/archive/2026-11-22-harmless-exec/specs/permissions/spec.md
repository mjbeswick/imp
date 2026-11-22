## MODIFIED Requirements

### Requirement: Safe-listed commands run unprompted

The system SHALL run a command without confirmation in `normal` only when every pipeline, alone or joined by `;`, `&&`, or `||`, starts with a safe-listed read-only program, SHALL ask for a lone `&`, a substitution, or a write redirect, SHALL treat quoted text as an argument except `$(` and backticks in double quotes, SHALL ask when a safe program gets a flag that writes or runs, and SHALL judge every `find -exec` or `-execdir` clause by the same rule, asking when it has no terminator.

#### Scenario: Inspection command
- **WHEN** the model runs `ls -la`, `rg TODO`, or `git diff`
- **THEN** each runs without a prompt

#### Scenario: Unlisted command
- **WHEN** the model runs `npm install`
- **THEN** a confirmation is required

#### Scenario: Safe pipe
- **WHEN** the model runs `rg TODO src | head -20`
- **THEN** it runs without a prompt

#### Scenario: Safe exec clause
- **WHEN** the model runs `find . -type f -exec ls -l {} +`
- **THEN** it runs without a prompt

#### Scenario: Exec runs a writer
- **WHEN** the model runs `find . -exec rm {} +` or `find . -exec sort -o out {} +`
- **THEN** a confirmation is required

#### Scenario: Exec runs a shell
- **WHEN** the model runs `find . -exec sh -c 'echo hi' ;`
- **THEN** a confirmation is required

#### Scenario: Exec deletes
- **WHEN** the model runs `find . -delete` or `find . -exec find . -delete ;`
- **THEN** a confirmation is required

#### Scenario: Exec without a terminator
- **WHEN** the command is `find . -exec ls`
- **THEN** a confirmation is required


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

The system SHALL offer `s` and `a` only on a confirmation for one simple, unprivileged command in `normal`, or one piped into safe filters, judged by its first command, SHALL NOT offer either for a command that runs another program through `-exec`, `-execdir`, `--exec`, or `xargs`, SHALL on `s` skip that program and its first argument for the session, SHALL on `a` save an `allow` line for all sessions, SHALL apply `allow` only in `normal`, and SHALL forget `s` approvals on `/new` and on exit.

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

#### Scenario: No always for exec
- **WHEN** the command is `find . -exec rm {} +` or `some | xargs rm`
- **THEN** the prompt offers only `y` and `n`

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
- **WHEN** the user answers `s` and then starts `/new`
- **THEN** the same command asks again


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
