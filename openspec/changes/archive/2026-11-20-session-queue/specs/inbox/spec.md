## REMOVED Requirements

### Requirement: The inbox lists every live session

**Reason**: A list to manage is replaced by a queue to step through.
**Migration**: Ctrl+O or `/next` goes to the next waiting session.

### Requirement: Act from the inbox

**Reason**: Approvals are answered in the session itself after Ctrl+O lands there.
**Migration**: Ctrl+O, then `y`, `n`, `s`, or `a` at the prompt.

## ADDED Requirements

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
