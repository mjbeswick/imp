## MODIFIED Requirements

### Requirement: Safe-listed commands run unprompted

The system SHALL run a command without confirmation in `normal` only when every part of every pipeline, alone or joined by `;`, `&&`, or `||`, starts with a safe-listed program for listing, inspection, search, or read-only git, SHALL ask for a lone `&`, a redirect, or a substitution outside quotes, SHALL treat quoted text as an argument except `$(` and backticks inside double quotes, and SHALL ask when a safe program gets a flag that writes files or runs commands.

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
