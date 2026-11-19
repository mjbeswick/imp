# Spec Delta

## ADDED Requirements

### Requirement: A root change is reported once

The system SHALL report a successful session root change with one line that names the new root, with the home directory as `~`, and the prompt cache reset, and SHALL NOT repeat it.

#### Scenario: Choose a folder
- **WHEN** the user chooses `~/Projects/imp` in the `/cd` browser
- **THEN** the transcript gains exactly one line, `cwd: ~/Projects/imp (prompt cache reset)`

#### Scenario: Typed path
- **WHEN** the user runs `/cd ~/Projects/imp`
- **THEN** the same single line appears
