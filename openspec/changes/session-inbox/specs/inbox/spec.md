# Spec Delta

## ADDED Requirements

### Requirement: Several sessions run at once

The system SHALL keep up to four live sessions in one process, each with its own worker, transcript, mode, approvals, and jobs, SHALL start another live session with `/new` and when resuming from `/sessions`, SHALL close the oldest idle session to make room when four are live, and SHALL refuse with a message when all four are working or waiting.

#### Scenario: Second session
- **WHEN** a turn is running and the user runs `/new`
- **THEN** a new empty session opens and the first keeps running

#### Scenario: Room made
- **WHEN** four sessions are live, one is idle, and the user runs `/new`
- **THEN** the idle one closes, stays in `/sessions`, and the new one opens

#### Scenario: No room
- **WHEN** four sessions are working or waiting and the user runs `/new`
- **THEN** the transcript says all four are busy and nothing closes

#### Scenario: Separate approvals
- **WHEN** the user approved `wttr.in` in one session
- **THEN** a fetch from `wttr.in` in another session still asks

### Requirement: Switch sessions by key

The system SHALL move to the next live session with Ctrl+N and the previous with Ctrl+P, wrapping at the ends, SHALL keep each session's composer draft and scroll position when switching, and SHALL show the session's position and title in the status line while more than one is live.

#### Scenario: Next session
- **WHEN** two sessions are live and the user presses Ctrl+N
- **THEN** the other session's transcript and draft appear

#### Scenario: One session
- **WHEN** only one session is live
- **THEN** Ctrl+N and Ctrl+P do nothing and the status line is unchanged

### Requirement: The inbox lists every live session

The system SHALL open the inbox with Ctrl+O or `/inbox` and close it with Ctrl+O or Esc, SHALL show one row per live session with its title, state (`needs approval`, `replied`, `working`, or `idle`), the last line of its reply or its pending question, and its age, and SHALL order rows by that state list and then newest first.

#### Scenario: Open
- **WHEN** three sessions are live and the user presses Ctrl+O
- **THEN** the inbox shows three rows and the highlight is on the first

#### Scenario: Order
- **WHEN** one session waits for approval and another has replied
- **THEN** the waiting one is first

#### Scenario: Replied clears
- **WHEN** the user opens a session marked `replied`
- **THEN** it becomes `idle`

### Requirement: Act from the inbox

The system SHALL open the highlighted session with Enter, SHALL answer its pending confirmation with `y`, `n`, or `a` without opening it, following the same rules as the prompt, SHALL close an idle session with `x`, and SHALL show `stop it first` when `x` is pressed on a working or waiting session.

#### Scenario: Approve in place
- **WHEN** a row shows `needs approval` for `run npm test` and the user presses `y`
- **THEN** the command runs in that session, the inbox stays open, and the row becomes `working`

#### Scenario: Open a session
- **WHEN** the user presses Enter on a row
- **THEN** the inbox closes and that session is in view

#### Scenario: Close a session
- **WHEN** the user presses `x` on an idle row
- **THEN** the session leaves the inbox and stays in `/sessions`

### Requirement: Background sessions ask for attention

The system SHALL count sessions out of view that need approval or have replied and show the count in the status line, SHALL ring the bell once when such a session first needs approval, and SHALL NOT apply the confirmation deadline to a session out of view, which waits until answered.

#### Scenario: Count
- **WHEN** two sessions out of view need approval
- **THEN** the status line shows `2 waiting`

#### Scenario: No deadline out of view
- **WHEN** a session out of view needs approval for ten minutes
- **THEN** the confirmation is still open and nothing ran

#### Scenario: Deadline in view
- **WHEN** the user switches to that session
- **THEN** the prompt shows and its deadline starts from then
