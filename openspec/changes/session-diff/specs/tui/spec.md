## RENAMED Requirements

- FROM: `### Requirement: Twenty-three slash commands, no more`
- TO: `### Requirement: Twenty-five slash commands, no more`

## MODIFIED Requirements

### Requirement: Twenty-five slash commands, no more

The system SHALL provide exactly `/help`, `/keys`, `/diff`, `/new`, `/goal`, `/copy`, `/sessions`, `/agent`, `/reload`, `/compact`, `/think`, `/mode`, `/next`, `/verbosity`, `/cwd`, `/cd`, `/image`, `/fork`, `/theme`, `/shell`, `/model`, `/provider`, `/queue`, `/rename`, and `/quit`, SHALL run them without sending a request to the model, and SHALL NOT provide `/clear`, `/yolo`, or `/cost`.

#### Scenario: Help
- **WHEN** the user types `/help`
- **THEN** all twenty-three commands appear with a one-line description

#### Scenario: Unknown command
- **WHEN** the user types `/nope`
- **THEN** nothing is sent to the model and the nearest matching command is suggested

#### Scenario: Rename
- **WHEN** the user runs `/rename fix the parser`
- **THEN** the current session carries that title and no request is sent

#### Scenario: Line mode
- **WHEN** the user runs `/help` in line mode
- **THEN** it prints the same list as a plain block

## ADDED Requirements

### Requirement: The session's changes can be shown

The system SHALL show, for `/diff`, each file imp's `edit` and `write` tools changed in the session with its added and removed line counts, followed by its net diff against the version before imp first touched it, SHALL style each diff with the syntax rules of `diff-style` by its file's extension, SHALL note that shell commands' changes are not tracked, SHALL run `git diff` with the given arguments in the session directory for `/diff git [args]`, and SHALL send neither to the model.

#### Scenario: Two edits, one change
- **WHEN** imp edited `calc.py` twice in the session and the user runs `/diff`
- **THEN** one summary row names `calc.py` with its counts, and one diff shows the combined change

#### Scenario: New file
- **WHEN** imp wrote `notes.md`, which did not exist before
- **THEN** its diff shows every line as added

#### Scenario: Nothing changed
- **WHEN** imp has changed no file this session
- **THEN** `/diff` says so and suggests `/diff git`

#### Scenario: Git
- **WHEN** the user runs `/diff git --staged` in a repository
- **THEN** the staged changes show as diff rows and the model receives nothing

#### Scenario: Syntax
- **WHEN** `/diff` or `/diff git` shows a change to `main.rs`
- **THEN** its rows carry Rust syntax styling, and a change to `notes.txt` in the same output carries none
