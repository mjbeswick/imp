## ADDED Requirements

### Requirement: Each process has a private scratch folder

The system SHALL create a scratch folder per process under its state directory, SHALL pass its path to agent commands as `IMP_TMP`, SHALL name it in the prompt, SHALL let read, write, and edit reach it, SHALL remove it when the process exits, and SHALL remove at start the folders of processes that no longer run.

#### Scenario: Agent writes scratch
- **WHEN** the agent writes `$IMP_TMP/notes.txt` in normal mode
- **THEN** the write succeeds without a prompt

#### Scenario: Exit cleans up
- **WHEN** imp exits
- **THEN** its scratch folder no longer exists

#### Scenario: Live folders stay
- **WHEN** imp starts while another imp process runs
- **THEN** the other process's scratch folder is kept
