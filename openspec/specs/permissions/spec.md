# permissions Specification

## Purpose
Decides, for every action the agent takes, whether it runs, asks first, or is refused. One setting, three modes, one safe list, one deny list.

## Requirements

### Requirement: One setting selects the policy

The system SHALL read a single `permissions` setting with values `normal`, `strict`, or `yolo`, defaulting to `normal`, and SHALL report the active mode at startup.

#### Scenario: Default mode
- **WHEN** the config has no permissions setting
- **THEN** the session starts in `normal` and says so

#### Scenario: Unknown value
- **WHEN** the setting is not one of the three values
- **THEN** the session refuses to start and names the valid values

### Requirement: Reads never prompt

The system SHALL allow `read` and `search` without confirmation in every mode.

#### Scenario: Reading a file
- **WHEN** the model reads any file inside the jail
- **THEN** no prompt appears in any mode

### Requirement: Safe-listed commands run unprompted

The system SHALL run commands whose first word matches its safe list without confirmation in `normal` mode, covering listing, inspection, search, and read-only git commands.

#### Scenario: Inspection command
- **WHEN** the model runs `ls -la`, `rg TODO`, or `git diff`
- **THEN** each runs without a prompt

#### Scenario: Unlisted command
- **WHEN** the model runs `npm install`
- **THEN** a confirmation is required

### Requirement: Everything else asks before acting

The system SHALL require confirmation for any file mutation outside the session directory and for any command not on the safe list, SHALL treat a refusal as a tool result rather than an error, and SHALL let the model continue.

#### Scenario: Approved mutation
- **WHEN** the model writes outside the session directory and the user approves
- **THEN** the write happens and the result is normal

#### Scenario: Refused mutation
- **WHEN** the user refuses
- **THEN** nothing changes and the model receives a result saying the user declined

#### Scenario: Strict mode
- **WHEN** the mode is `strict`
- **THEN** every file mutation and every command prompts, including safe-listed ones

### Requirement: Deny always wins

The system SHALL refuse actions matching its deny list in every mode, including `dd`, `mkfs`, shutdown commands, recursive `rm` targeting a path outside the session directory, recursive `chmod` on a root path, `git push --force`, `git reset --hard`, and any pipeline that pipes a download into a shell, and SHALL apply the list whether or not the command is prefixed with `sudo`.

#### Scenario: Denied command
- **WHEN** the model runs `dd if=/dev/zero of=/dev/disk0`
- **THEN** the call is refused without a prompt and the model is told why

#### Scenario: Sudo does not launder a denied command
- **WHEN** the model runs `sudo rm -rf /`
- **THEN** it is refused by the same rule that refuses `rm -rf /`

#### Scenario: Yolo
- **WHEN** the mode is `yolo` and the model runs `rm -rf /`
- **THEN** it is refused, and only the denials remain while the other prompts are gone

### Requirement: Superuser commands are opt-in

The system SHALL refuse `sudo` unless `allow_sudo=true`, defaulting to false, SHALL return a message naming the setting when refused, and when enabled SHALL require confirmation for every sudo command regardless of the safe list or the permission mode.

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
- **THEN** it is refused, and with `allow_sudo=true` it still asks

### Requirement: Password entry hands the terminal over

The system SHALL run a sudo command that needs a password by leaving the alternate screen and cooked-input mode, running the command attached to the terminal so the password can be typed, and then restoring the interface, and SHALL never capture, store, or forward the password.

#### Scenario: Cached credentials
- **WHEN** sudo credentials are already cached
- **THEN** the command runs without a prompt and the interface is not disturbed

#### Scenario: Password needed
- **WHEN** sudo asks for a password
- **THEN** the terminal returns to normal mode, the user types the password, the command completes, and the interface is redrawn

#### Scenario: Password declined
- **WHEN** the user presses Ctrl+C at the password prompt
- **THEN** the command is reported as cancelled and the interface is restored

### Requirement: The jail is independent of the mode

The system SHALL keep the path jail for file tools enabled in `normal` and `strict`, SHALL disable it only in `yolo`, and SHALL state clearly when it is disabled.

#### Scenario: Yolo warning
- **WHEN** the session starts in `yolo`
- **THEN** the interface says the path jail is off

#### Scenario: Jail in normal mode
- **WHEN** the model writes outside the session directory in `normal` mode
- **THEN** it prompts, and a refusal means no write occurs

### Requirement: Children inherit the policy

The system SHALL apply the parent's permission policy to every subagent, SHALL route their prompts to the root interface tagged with the agent name, and SHALL NOT let a child relax the policy.

#### Scenario: Child inherits
- **WHEN** the parent runs in `strict` and a child attempts a write
- **THEN** the prompt appears and the child waits

### Requirement: Guardrails are documented as guardrails

The system SHALL describe its permission rules as pattern-based guardrails rather than a security boundary, in the specification, the README, and the startup report.

#### Scenario: Honest reporting
- **WHEN** the user asks what the permissions protect against
- **THEN** the answer is that they slow down mistakes, and that a human is still the boundary for shell commands
