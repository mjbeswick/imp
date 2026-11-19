# Spec Delta

## MODIFIED Requirements

### Requirement: Shell commands run non-interactively

The system SHALL run `bash` commands through `sh -c` with stdin on the null device, a 30 second timeout, and a process group, SHALL return the exit code, and SHALL hand a model call still running at the timeout to the job table instead of terminating it.

#### Scenario: Prompting command
- **WHEN** a command would wait for terminal input
- **THEN** it reads end of file immediately instead of hanging the agent

#### Scenario: Timeout
- **WHEN** a model command runs longer than 30 seconds
- **THEN** it keeps running as a background job and the result names the job id, its log, and `job wait`

#### Scenario: Typed timeout
- **WHEN** a typed `!` command runs longer than 30 seconds
- **THEN** the process group is terminated and the result reports a timeout

#### Scenario: Cancel
- **WHEN** the user cancels while a command runs
- **THEN** the process group gets SIGTERM, then SIGKILL after two seconds, and the result reads cancelled

## ADDED Requirements

### Requirement: Bash calls can carry a label

The system SHALL accept an optional `description` on `bash`, SHALL show it as the tool row summary in place of the command, SHALL still show the command in expanded and high-verbosity rows, and SHALL name the command in any confirmation for a labelled call.

#### Scenario: Labelled call
- **WHEN** the model runs a long pipeline with the description `count photos by month`
- **THEN** the row reads `count photos by month` and Ctrl+O shows the command

#### Scenario: Confirmation
- **WHEN** a labelled call needs confirmation
- **THEN** the prompt names the command, not only the verb
