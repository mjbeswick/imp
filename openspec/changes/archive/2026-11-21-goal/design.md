# Design

## State

`Session.goal: Option<Goal { text, round, rounds }>`. It lives in the worker,
like check-ins; it is not saved, so a resumed session has no goal.

## Rounds

The first turn sends the user's goal as the message, with a system note:
"[Goal from imp, not the user.] Work until this goal is met: <text>. End your
reply with GOAL MET and the evidence when it is met, or GOAL BLOCKED: <why>
when you need the user. Otherwise keep working."

After each turn the worker calls `Session::goal_next(reply)`. It returns the
next note when the goal still runs, or a reason it ended. Each later round's
note is "[Goal check from imp, not the user.] Round n of N. Goal: <text>.
…", with the same endings. Markers are matched on the reply's last
non-empty line, ignoring case, so prose that mentions them does not count.

## Ends

- Met, blocked, or out of rounds: an info row says which, and the goal clears.
- Esc cancels the turn and clears the goal.
- A user message clears the goal before its turn.
- A declined confirmation sets a flag that `goal_next` reads.
- An error clears it.

## Cost

Rounds are capped, and the limit shows in the status line. A larger
`goal_rounds` is the user's choice; the maximum is 20.
