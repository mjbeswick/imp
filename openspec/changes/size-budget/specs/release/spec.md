## ADDED Requirements

### Requirement: The binary has a size budget

The size script SHALL fail when the release binary is larger than the limit recorded in the repository, SHALL print the size and the limit, and SHALL pass at or under it.

#### Scenario: Over budget
- **WHEN** the release binary is one byte over the limit
- **THEN** the script exits non-zero and prints both sizes

#### Scenario: Within budget
- **WHEN** the binary is at or under the limit
- **THEN** the script exits zero and reports the change since the last record
