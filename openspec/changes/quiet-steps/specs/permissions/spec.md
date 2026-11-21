## ADDED Requirements

### Requirement: Listing tracked files runs unprompted

The system SHALL run `git ls-files` with any arguments without asking, alone or in a safe chain or pipeline.

#### Scenario: Listing a project

- **WHEN** the agent runs `git ls-files | head -50`
- **THEN** it runs without a confirmation
