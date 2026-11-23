## ADDED Requirements

### Requirement: Reload re-reads the config

The system SHALL re-read the config file on `/reload`, SHALL apply each key whose value in the file changed since it was last read and list it as `key: old → new`, SHALL keep the session's value for every key the file did not change, SHALL name a key removed from the file as needing a restart, and SHALL name unknown keys once at startup and on reload without failing.

#### Scenario: Privilege setting
- **WHEN** the user adds `allow_sudo=true` to the config and runs `/reload`
- **THEN** a line reads `allow_sudo: false → true` and the next privileged command is classified as allowed

#### Scenario: Session choice kept
- **WHEN** the user picked a model with `/model`, then edits only `allow_sudo` and reloads
- **THEN** the picked model stays

#### Scenario: Unknown key
- **WHEN** the config contains a key imp does not know
- **THEN** startup names that key as unknown and the run continues
