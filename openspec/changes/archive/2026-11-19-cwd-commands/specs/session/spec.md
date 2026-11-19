# Spec Delta

## ADDED Requirements

### Requirement: Resume restores the recorded start directory

The system SHALL record the session root in the sidecar when a session starts and whenever `/cd` changes it, SHALL use that recorded directory as the root when the session is resumed, and SHALL let an explicit `--cwd` override it.

#### Scenario: Continue where it worked
- **WHEN** a session was changed to `/tmp` and the user later runs `imp -c`
- **THEN** the restored session is rooted at `/tmp`

#### Scenario: Explicit override
- **WHEN** the user runs `imp -c --cwd ~/Projects/imp` on that session
- **THEN** the root is `~/Projects/imp` and the sidecar keeps its recorded directory until `/cd` changes it

#### Scenario: Sidecar updated
- **WHEN** `/cd /tmp` runs in a session
- **THEN** its sidecar's start directory becomes `/tmp` without rewriting the transcript

#### Scenario: New session
- **WHEN** a session starts in `/tmp`
- **THEN** its sidecar records `/tmp` as the start directory

#### Scenario: Missing recorded directory
- **WHEN** the recorded directory no longer exists on resume
- **THEN** imp falls back to the launch directory and says so
