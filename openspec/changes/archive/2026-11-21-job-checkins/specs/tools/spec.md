## MODIFIED Requirements

### Requirement: Long commands run as background jobs

The system SHALL start a `bash` command as a job when `background` or `delay_s` is set, SHALL start it after `delay_s` seconds when given, SHALL return a job id and its output path immediately, SHALL cap `job wait` at 55 seconds and return partial output, SHALL show each job's elapsed time, output size, and last output line, and SHALL report a finished job to the model as a `system` note at the tail.

#### Scenario: Start and continue
- **WHEN** the model starts `make` in the background
- **THEN** it gets a job id at once and the turn can continue

#### Scenario: Wait with a budget
- **WHEN** the model waits on a running job
- **THEN** it receives output produced so far and either an exit code or "still running"

#### Scenario: Completion note
- **WHEN** a background job exits between turns
- **THEN** a system note records the job id, exit code, and output path

#### Scenario: Kill
- **WHEN** the model kills a job
- **THEN** its process group receives SIGTERM then SIGKILL

#### Scenario: Jobs do not survive a restart
- **WHEN** the process exits with a job running
- **THEN** no record of that job exists in the next session

#### Scenario: Delayed start
- **WHEN** the model runs `rsync -a src/ dst/` with `delay_s` 3600
- **THEN** the job is listed as starting in an hour and nothing runs until then

#### Scenario: Cancelled before it starts
- **WHEN** the model kills a delayed job before its delay ends
- **THEN** the command never runs

#### Scenario: Progress
- **WHEN** a running job has printed `1,234,567  45%  12.0MB/s`
- **THEN** `job list` shows its elapsed time, output size, and that line

## ADDED Requirements

### Requirement: Agents can check back on jobs

The system SHALL let the model schedule a check-in with `job check` and `after_s` between 30 seconds and 24 hours, optionally for one job, SHALL let `bash` set `notify` to be told when that job ends, SHALL wake the session at that time with a turn that opens with a note from imp, not the user, giving the job's state and latest output, SHALL keep at most four pending check-ins per session, SHALL drop them when the session ends, and SHALL refuse them in line mode.

#### Scenario: Check-in
- **WHEN** the model starts a copy as job 3 and runs `job check` with `after_s` 600
- **THEN** ten minutes later a turn starts with a note naming job 3, its elapsed time, and its last output line

#### Scenario: Told when done
- **WHEN** the model starts a job with `notify` and the job exits while the session is idle
- **THEN** a turn starts with a note giving the exit code and the output's tail

#### Scenario: Limits
- **WHEN** four check-ins are pending, or `after_s` is under 30
- **THEN** the check is refused with the limit named

#### Scenario: Line mode
- **WHEN** the model runs `job check` in line mode
- **THEN** it is refused and told check-ins need the chat
