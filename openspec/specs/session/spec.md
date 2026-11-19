# session Specification

## Purpose
Records each session on disk, lets the user start, continue, pick, rename, and delete sessions from the interface, and restores a resumed session with the exact context the model saw.

## Requirements

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
- **WHEN** the user runs `imp -c`
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
- **WHEN** the user runs `imp --list`
- **THEN** a table of sessions prints and the process exits without opening the interface

#### Scenario: Scripted rename
- **WHEN** the user runs `imp --rename 20261117-1042 "flaky rsync"`
- **THEN** the sidecar title changes and the transcript is untouched

#### Scenario: Scripted delete
- **WHEN** the user runs `imp --delete 20261117-1042`
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

### Requirement: Sessions can be forked

The system SHALL copy the running session's transcript and sidecar into a new session when `/fork` runs, SHALL give the copy a new id and a title naming the source, SHALL leave the source transcript and sidecar unchanged, SHALL attach the copy to the running session so later turns append to it, and SHALL report both ids.

#### Scenario: Fork copies the history
- **WHEN** the user runs `/fork` after several turns
- **THEN** a new transcript exists with the same bytes as the source and the session continues in the copy

#### Scenario: Source survives
- **WHEN** the fork exists
- **THEN** the original session still appears in `/sessions` with its own history unchanged

#### Scenario: Title
- **WHEN** the source is titled "flaky rsync test"
- **THEN** the fork is titled "fork of flaky rsync test"

#### Scenario: Exit id
- **WHEN** the user quits after forking
- **THEN** the printed resume id is the fork's id, not the source's

#### Scenario: Line mode
- **WHEN** the user runs `/fork` in line mode
- **THEN** both ids print and the next message appends to the fork

#### Scenario: No session yet
- **WHEN** `/fork` runs before a session log exists
- **THEN** the transcript reports that there is nothing to fork and nothing changes

### Requirement: Resume shows the restored transcript

The system SHALL draw a resumed session's user messages, assistant replies, and one finished row per tool call in the chat interface, SHALL omit reasoning and system messages, SHALL replace the previous transcript, SHALL show the restored context's token estimate in the status line, and SHALL count cost only for messages sent in the current process. This SHALL hold for `-r`, `-c`, and the session picker.

#### Scenario: Resume from the shell
- **WHEN** the user runs `imp -r <id>` for a session with three exchanges
- **THEN** the transcript shows the three user messages and replies in order and the status line shows a non-zero token count

#### Scenario: Resume from the picker
- **WHEN** the user picks a session while another conversation is on screen
- **THEN** the old transcript is replaced by the chosen session's

#### Scenario: Tool calls
- **WHEN** the resumed history holds a tool call
- **THEN** one finished tool row with its summary appears where the call was made

#### Scenario: Cost starts fresh
- **WHEN** a session is resumed and nothing is sent
- **THEN** the cost shown is zero

### Requirement: Session tables share one layout

The system SHALL print sessions in `--list`, line-mode `/sessions`, and the session picker with the same fields in the same order: id, age, title, and message count, SHALL show the title on one line by turning newlines into spaces, SHALL show `(untitled)` for an empty title, SHALL cut a title that does not fit and keep the count whole and aligned, and SHALL show the newest twenty in line-mode `/sessions` as the picker does.

#### Scenario: Line mode list
- **WHEN** the user runs `/sessions` in line mode with sixty sessions stored
- **THEN** twenty rows print with ages, and every count ends in one column

#### Scenario: Multi-line title
- **WHEN** a session title holds a newline
- **THEN** its row stays on one line

### Requirement: Session delete confirmation names the session

The system SHALL ask to delete a session by its title and id, and SHALL keep the session picker on screen behind the question.

#### Scenario: Delete from the picker
- **WHEN** the user presses `d` on `Rate my machine.`
- **THEN** the prompt reads `delete "Rate my machine." (51280befc)? y/N` and the list stays visible
