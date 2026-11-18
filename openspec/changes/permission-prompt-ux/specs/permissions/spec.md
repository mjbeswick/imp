# Spec Delta

## MODIFIED Requirements

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
