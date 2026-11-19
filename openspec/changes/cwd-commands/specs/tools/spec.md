# Spec Delta

## MODIFIED Requirements

### Requirement: Shell commands run in the session root

The system SHALL start every `bash` command in the session root, SHALL take that root from `-C, --cwd <PATH>` when given, from `/cd` when entered, and from the launch directory otherwise, SHALL resolve a relative `--cwd` against the launch directory and a relative `/cd` against the current root, SHALL refuse a missing or non-directory value before any request is sent, and SHALL move the path jail and typed `!` commands to the same directory.

#### Scenario: Default root
- **WHEN** the user runs `imp` from a project directory and the model runs `pwd`
- **THEN** the command prints that project directory

#### Scenario: Chosen root
- **WHEN** the user runs `imp --cwd ~/Projects/imp` and the model runs `ls`
- **THEN** the listing is of `~/Projects/imp`, not the launch directory

#### Scenario: Changed root
- **WHEN** the user runs `/cd /tmp` and the model then runs `pwd`
- **THEN** the command prints `/tmp`

#### Scenario: Relative root
- **WHEN** the user runs `imp -C ..` from `~/Projects/imp`
- **THEN** the session root is `~/Projects`

#### Scenario: Missing directory
- **WHEN** the user runs `imp --cwd /nope`
- **THEN** imp exits with an error naming the path and no session starts

#### Scenario: Typed shell command
- **WHEN** the user types `!pwd` at the prompt
- **THEN** it prints the same root the model's `bash` calls use
