# Tasks

Extracted from `build-mote-v1` when it was archived. Verification only:
tests, fixtures, and two known gaps. No new behaviour.

## 3. Agent loop

- [x] 3.2 Cancel: shared flag checked between chunks and before each tool call; turn ends as cancelled. Verify: fixture that pauses mid-stream ends within one second of the flag.
- [x] 3.3 One-retry policy with backoff under two seconds. Verify: exactly two requests for a retryable failure, one for a terminal one.
- [x] 3.4 Thinking control: disabled by default, per-turn enable, effort override, reasoning echoed on later requests. Verify: snapshot of request two shows prior reasoning; a test shows no `temperature` field when thinking is on.

## 4. Tools

- [x] 4.11 Privilege gating: refused unless `allow_sudo=true`, always prompts when enabled, deny list applies with or without the prefix. Verify: default refusal names the setting; an approved sudo command runs; `sudo rm -rf /` is refused.
- [x] 4.12 Sudo terminal handoff: leave the alternate screen and raw mode, run attached to the tty, restore and redraw after. Verify: cached credentials skip the handoff; a password prompt works against a scratch sudoers rule; Ctrl+C at the prompt cancels cleanly and the screen is redrawn.

## 5. TUI

- [x] 5.2 ANSI renderer: clear, wrap, cursor placement, colour escapes, no-colour fallback, full redraw within one frame budget. Verify: byte-level snapshot of one frame at 100x30.
- [x] 5.2a Colour roles and palettes: six semantic roles, dark and light tables, truecolor and 256-colour variants, `NO_COLOR` and `color=never` parity. Verify: snapshot of the same frame in four colour modes, plus a no-colour test asserting zero escape bytes.
- [x] 5.3 Streaming render with tail-follow, scroll detach, unseen-line count. Verify: snapshots after 100 appended tokens and after scrolling up.
- [x] 5.4 Thinking line collapsed by default, expandable inline, token count. Verify: snapshots before and after expansion.
- [x] 5.5 Tool lines: name, truncated arguments, duration, error colour, expand-last-output inline. Verify: snapshots for success and non-zero exit.
- [x] 5.6 Status line: model, context estimate, cache hit share, cost, unknown-model fallback, one dim row only. Verify: unit tests over usage structs including a zero-hit response.
- [x] 5.7 Key parsing from raw bytes: the full key set, including the double Ctrl+C guard. Verify: input-event tests per key.
- [x] 5.8 Question picker: numbered choices, custom-answer row, typed answer echo, dismissal, and `y`/`n` shortcuts for a two-option yes/no question. Verify: snapshot and key-handling tests for pick, type, dismiss, and both y/n answers.

## 6. Context, prune, compaction

- [x] 6.2 Token estimator and thresholds. Verify: unit tests either side of 45k and 60k, and a test that the mid-loop case defers compaction.
- [x] 6.3 Prune: older tool results collapse to a stub plus spill path at 45k, structure intact, no model call. Verify: a 70k history prunes below 60k and compaction does not run.
- [x] 6.4 Compaction: four exchanges verbatim including reasoning and tool calls, summarizer with thinking disabled, one `system` message named `compacted`, before/after reporting. Verify: a 20-exchange history compacts to a summary plus four exchanges.
- [x] 6.5 Drop orphan tool results after range removal. Verify: a result whose call was summarized is excluded.
- [x] 6.6 Tail notes: memory writes and skill loads mid-session arrive as `system` messages without touching the prefix. Verify: prefix bytes identical either side of a memory save.
- [x] 6.7 Mid-loop `context_length_exceeded`: truncate older tool results, retry once, then fail the turn. Verify: one retry is issued and the second failure is a clean error.

## 7. Memory

- [x] 7.5 Permission behaviour for memory writes in both scopes. Verify: a global save prompts in `normal`, a project save does not, and a refusal leaves the directory byte-identical.

## 8. Skills and agents

- [x] 8.2 Skill index in the prompt, one line each, capped at 40. Verify: twenty skills add twenty short lines.
- [x] 8.5 `--agent` at launch and `/agent` picker mid-conversation, keeping the transcript. Verify: switching changes the tool list and instructions, and switching back restores both.
- [x] 8.6 Switch delivery as a tail `system` note that supersedes prior instructions, with no prefix rewrite and a reported no-reset. Verify: prefix hash identical either side of a switch.
- [x] 8.7 Record the switch in the session log and restore the active agent on resume. Verify: round-trip a session with one switch.

## 10. Sessions

- [x] 10.4 `-c`, `-r <id>`, explicit path, and the in-app picker with newest first and a "new session" entry. Verify: each entry point loads the intended file; the picker test uses a fixture directory.
- [x] 10.4a CLI management: `--list` prints id, title, age, and message count then exits; `--rename <id> <title>`; `--delete <id>` with confirmation. Verify: a fixture directory with three sessions, asserting output columns, exit status, and that transcripts are byte-identical after a rename.
- [x] 10.4b Picker actions: Enter opens, `n` starts new, `r` renames, `d` deletes after confirmation. Verify: key-event tests for each action against a fixture directory.

## 11. Polish and proof

- [x] 11.1 Recorded-fixture replay harness so the whole loop runs offline. Verify: `cargo test` passes with networking disabled.
- [x] 11.3 Measure and record: binary size, RSS, cold start, cache hit share over ten turns. Verify: numbers in `README.md` compared with the budget.
- [x] 11.4 Mode selection: `--cli`, `--chat`, and the tty-based default, with the line mode emitting no escape sequences. Verify: a scripted run with stdout piped contains zero ESC bytes, and `--chat` in a terminal opens the alternate screen.
- [x] 11.5 Ship a starter skill set for common CLIs (git, rg, jq, gh, docker, macOS) with routing descriptions, reusing the existing llm-functions skill files where they fit. Verify: each skill loads, and a request naming the tool routes to it.

## 12. Close the spec gaps

- [x] 12.4 Mid-loop `context_length_exceeded`: truncate old tool results, retry once, then fail cleanly. Verify: a forced 400 with that body produces one retry and a second failure ends the turn.
- [x] 12.7 `/sessions` loads the chosen session in place. Verify: choosing an entry replaces the transcript and continues that session.
