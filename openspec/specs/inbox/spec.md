# inbox Specification

## Purpose
Several imp sessions run side by side in one process, and one key walks
the user through the ones waiting on them, oldest first, so nobody juggles
terminal tabs to find the session waiting for a `y`.

## Requirements

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
- **WHEN** the user answered `s` to `npm test` in one session
- **THEN** `npm test` in another session still asks

### Requirement: Switch sessions by key

The system SHALL move to the next live session with Ctrl+N and the previous with Ctrl+P, wrapping at the ends, SHALL keep each session's composer draft and scroll position when switching, and SHALL show the session's position and title in the status line while more than one is live.

#### Scenario: Next session
- **WHEN** two sessions are live and the user presses Ctrl+N
- **THEN** the other session's transcript and draft appear

#### Scenario: One session
- **WHEN** only one session is live
- **THEN** Ctrl+N and Ctrl+P do nothing and the status line is unchanged

### Requirement: Background sessions ask for attention

The system SHALL count sessions out of view that need approval or have replied and show the count in the status line, and SHALL ring the bell once when such a session first needs approval.

#### Scenario: Count
- **WHEN** two sessions out of view need approval
- **THEN** the status line shows `2 waiting`

#### Scenario: No deadline out of view
- **WHEN** a session out of view needs approval for ten minutes
- **THEN** the confirmation is still open and nothing ran

#### Scenario: Deadline in view
- **WHEN** the user switches to that session
- **THEN** the prompt shows, still open

### Requirement: Ctrl+O goes to the next waiting session

The system SHALL bring into view, on Ctrl+O or `/next`, the session that has waited longest for the user, where a session waits from when an approval or question opens or a reply finishes out of view, SHALL count a reply as read once it has been in view, SHALL move an unanswered approval or question in view to the back of the queue without answering it, and SHALL say `nothing waiting` when the queue is empty.

#### Scenario: Oldest first
- **WHEN** session 2 replied, then session 3 asked for approval, and the user presses Ctrl+O in session 1
- **THEN** session 2 comes into view

#### Scenario: Dismiss by moving on
- **WHEN** the user presses Ctrl+O on a reply and then Ctrl+O again without typing
- **THEN** the reply counts as read and the next waiting session comes into view

#### Scenario: Approval kept
- **WHEN** the user presses Ctrl+O on an unanswered approval
- **THEN** the approval stays open, goes to the back of the queue, and the next waiting session comes into view

#### Scenario: Empty queue
- **WHEN** nothing waits and the user presses Ctrl+O
- **THEN** the view stays and the transcript says `nothing waiting`

### Requirement: Moving on after handing off

The system SHALL, when `auto_next` is on, which is the default, bring the next waiting session into view after the user sends a message or answers an approval or question, SHALL say which session and why in one line, and SHALL NOT switch at any other time.

#### Scenario: After a reply
- **WHEN** session 2 waits and the user sends a message in session 1
- **THEN** session 2 comes into view with the line `→ 2/2 <title> · replied`

#### Scenario: Off
- **WHEN** `auto_next=off` and the user sends a message while another session waits
- **THEN** the view stays

#### Scenario: Typing
- **WHEN** a session starts waiting while the user is typing
- **THEN** the view does not change

### Requirement: A check-in wakes its session

The system SHALL show a check-in in its session's transcript as an information line, SHALL mark the session working while the woken turn runs, and SHALL treat the turn's end as a reply, so a session out of view joins the queue.

#### Scenario: Out of view
- **WHEN** a check-in fires in a session out of view and its turn ends
- **THEN** the session is in the Ctrl+O queue and the status line counts it as waiting

#### Scenario: In view
- **WHEN** a check-in fires in the session in view
- **THEN** the check-in line appears and the reply streams below it
