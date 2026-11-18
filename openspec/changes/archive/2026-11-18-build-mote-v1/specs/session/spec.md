# Spec Delta

## Purpose

Records each session on disk, lets the user start, continue, pick, rename, and delete sessions from the interface, and restores a resumed session with the exact context the model saw.

## ADDED Requirements

### Requirement: Every session is logged as JSONL

The system SHALL append one JSON record per message, tool call, tool result, compaction event, and usage sample to a session file named for the start time, and SHALL flush after each record so a crash loses at most one.

#### Scenario: Crash mid-turn
- **WHEN** the process is killed during a tool call
- **THEN** the session file contains every record written before that call

#### Scenario: Record content
- **WHEN** an assistant message with reasoning and a tool call is stored
- **THEN** the record contains the content, the reasoning content, and the tool call identifiers and arguments

### Requirement: Sessions carry a small sidecar

The system SHALL write one sidecar file per session holding its title, start directory, model, message count, and token totals, and SHALL read the list from sidecars rather than parsing full transcripts.

#### Scenario: Listing is cheap
- **WHEN** the user lists a thousand sessions
- **THEN** only sidecar files are read

#### Scenario: Sidecar missing
- **WHEN** a transcript exists without a sidecar
- **THEN** it still appears in the list with defaults derived from the file

### Requirement: Sessions have titles

The system SHALL title a new session from its first user message, truncated to a readable length, and SHALL allow the title to be changed without touching the transcript.

#### Scenario: Automatic title
- **WHEN** a session starts with "fix the flaky rsync test"
- **THEN** the list shows that as its title

#### Scenario: Rename
- **WHEN** the user renames a session
- **THEN** only the sidecar changes and the transcript is untouched

### Requirement: The user can start, continue, and pick sessions

The system SHALL start a new session by default, continue the most recent with `-c`, resume a named one with `-r <id>`, and open a picker in the interface that lists sessions newest first and starts a new one on request.

#### Scenario: Continue
- **WHEN** the user runs `mote -c`
- **THEN** the newest session is loaded and the input is ready

#### Scenario: Picker
- **WHEN** the user opens the session picker
- **THEN** sessions appear newest first with title, age, and message count, and choosing one loads it

#### Scenario: New from the picker
- **WHEN** the user chooses "new session" in the picker
- **THEN** a fresh session starts and the previous file stays intact

### Requirement: Sessions can be listed from the shell

The system SHALL provide `-l`/`--list` to print sessions newest first with id, title, age, and message count, and exit without starting the interface, and SHALL accept `--rename <id> <title>` and `--delete <id>` for scripted use.

#### Scenario: List and exit
- **WHEN** the user runs `mote --list`
- **THEN** a table of sessions prints and the process exits without opening the interface

#### Scenario: Scripted rename
- **WHEN** the user runs `mote --rename 20261117-1042 "flaky rsync"`
- **THEN** the sidecar title changes and the transcript is untouched

#### Scenario: Scripted delete
- **WHEN** the user runs `mote --delete 20261117-1042`
- **THEN** the session moves to trash after confirmation

### Requirement: The picker manages sessions in place

The system SHALL let the picker list sessions newest first, open one on Enter, start a new one with `n`, rename the highlighted one with `r`, and delete the highlighted one with `d` after confirmation.

#### Scenario: Rename in the picker
- **WHEN** the user presses `r`, types a title, and confirms
- **THEN** the list shows the new title and the transcript is untouched

#### Scenario: Delete in the picker
- **WHEN** the user presses `d` and confirms
- **THEN** the entry disappears from the list and its files are in trash

#### Scenario: Open
- **WHEN** the user selects a session and presses Enter
- **THEN** it loads and the input is ready

### Requirement: Deleting a session is reversible

The system SHALL move a deleted session's files to a trash directory rather than unlinking them, and SHALL require confirmation.

#### Scenario: Delete
- **WHEN** the user deletes a session and confirms
- **THEN** its transcript and sidecar move to the trash directory and no longer appear in the list

### Requirement: Resume rebuilds the wire history

The system SHALL restore a resumed session's request history exactly, including assistant reasoning content, tool call identifiers, matching tool results, and the compaction summary, so the next request is valid under the API's rules.

#### Scenario: Resume after compaction
- **WHEN** the user resumes a session that was compacted
- **THEN** the next request contains the summary message and the retained exchanges, and does not contain the summarized ones

#### Scenario: Resume never sends orphans
- **WHEN** a tool result in the log has no matching tool call
- **THEN** it is excluded from the restored history

### Requirement: Resume continues the same file

The system SHALL append to the resumed session's file rather than creating a new one, and SHALL support naming a specific file as well as picking the most recent.

#### Scenario: Continued session
- **WHEN** a session is resumed and the user sends a message
- **THEN** the new records are appended to the original file

#### Scenario: Explicit file
- **WHEN** the user passes a path to a session file
- **THEN** that file is loaded regardless of age

### Requirement: Children do not create sessions

The system SHALL log subagent activity inside the parent session's file as nested records or not at all, and SHALL NOT create a session file per subagent.

#### Scenario: Ten subagent runs
- **WHEN** a turn delegates ten times
- **THEN** the session list gains no new entries

### Requirement: Nothing leaves the machine

The system SHALL keep session logs, memory, and skills local, and SHALL make no network request other than the configured model endpoint.

#### Scenario: Audit of outbound traffic
- **WHEN** a session runs with memory and skills configured
- **THEN** the only outbound connections are to the model and search endpoints
