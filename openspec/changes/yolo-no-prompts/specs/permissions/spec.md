# Spec Delta

## MODIFIED Requirements

### Requirement: Superuser commands are opt-in

The system SHALL refuse `sudo` unless `allow_sudo=true`, defaulting to false, SHALL return a message naming the setting when refused, and when enabled SHALL require confirmation for every sudo command regardless of the safe list, except in `yolo`, where it runs without a prompt.

#### Scenario: Disabled by default
- **WHEN** the model runs `sudo systemsetup -setremotelogin off` with the default config
- **THEN** it is refused and the result names `allow_sudo`

#### Scenario: Enabled and approved
- **WHEN** `allow_sudo=true`, the model runs a sudo command, and the user approves
- **THEN** it runs and its output is captured under the normal caps

#### Scenario: Typed sudo
- **WHEN** the user types `!sudo launchctl list` with `allow_sudo=true`
- **THEN** it runs without a separate confirmation, having been typed

#### Scenario: Yolo does not grant sudo
- **WHEN** the mode is `yolo` with `allow_sudo=false` and the model runs `sudo ls`
- **THEN** it is refused, and with `allow_sudo=true` it runs without a prompt

## ADDED Requirements

### Requirement: Yolo never prompts

The system SHALL NOT ask for confirmation in `yolo` before a command, a file write, a memory save, or a change to imp's own config, SHALL still refuse anything on the deny list, and SHALL still let the model ask the user a question.

#### Scenario: Memory save
- **WHEN** the mode is `yolo` and the model saves a note
- **THEN** the note is written and no confirmation appears

#### Scenario: Deny list
- **WHEN** the mode is `yolo` and the model runs `mkfs`
- **THEN** it is refused without a prompt

### Requirement: imp's own config is the user's

The system SHALL refuse agent writes and shell commands that change imp's own config file in `normal` and `strict`, and SHALL allow them without a prompt in `yolo`.

#### Scenario: Normal
- **WHEN** the mode is `normal` and the model writes the config file
- **THEN** the write is refused with a reason

#### Scenario: Yolo
- **WHEN** the mode is `yolo` and the model writes the config file
- **THEN** the write happens without a prompt
