## ADDED Requirements

### Requirement: Dangling links cannot leave the jail

The system SHALL treat a symlink whose target does not exist as present when checking a path against the jail, and SHALL refuse a path through it rather than create the target.

#### Scenario: Dangling link
- **WHEN** the session directory holds `out -> /tmp/elsewhere/file`, the target does not exist, and the model writes `out` in `normal` mode
- **THEN** the write is refused and `/tmp/elsewhere/file` is not created
