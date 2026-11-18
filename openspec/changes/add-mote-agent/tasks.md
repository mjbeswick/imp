# Tasks

## 1. Skeleton and binary budget

- [x] 1.1 `cargo init --name mote`; add `serde_json` as the only crate; declare `extern "C"` bindings for libcurl and termios; set the release profile from design.md. Verify: `cargo build --release` succeeds and `ls -lh target/release/mote` is recorded.
- [x] 1.2 Hand-parse `~/.config/mote/config` as `key=value` with env overrides for key, model, base URL, permissions, no-colour. Verify: unit tests for comments, blank lines, unknown keys, and env precedence.
- [x] 1.3 Size check script failing over 1.5 MB. Verify: exits non-zero with the threshold lowered to 0.5 MB, and the real binary is under the limit.

## 2. HTTP and API client

- [x] 2.0 libcurl FFI: init, URL, headers, POST body, write callback, timeout, cleanup, error string. Verify: a request to a local test server streams chunks to the callback and reports a non-200 status correctly.
- [x] 2.1 Build request bodies with serde_json: model, messages, tools, `thinking`, `reasoning_effort`, `max_tokens`. Verify: snapshot of the exact JSON for a two-message history.
- [x] 2.2 Parse SSE from the write callback, buffering by line: `data:` chunks, `[DONE]`, keep-alive comments. Verify: fixture test yields expected text and final usage.
- [x] 2.3 Assemble tool-call deltas by `index`, id and name from the first fragment, empty names ignored. Verify: fixtures for split arguments, interleaved calls, and a late fragment with an empty name.
- [x] 2.4 Map errors and finish reasons to retryable and terminal outcomes. Verify: table test over 400/401/429/500, `aborted`, `insufficient_system_resource`, `length`, `content_filter`.
- [x] 2.5 Search and Fetch client for `/v1/tools/search` and `/v1/tools/fetch` with the same auth and retry rules. Verify: fixture tests for a five-result response, an empty result list, 404, and 429.

## 3. Agent loop

- [x] 3.1 Turn loop: stream, dispatch tools, feed results back, stop on no tool calls, stop at 24 iterations. Verify: offline fixture test with two tool calls then text.
- [ ] 3.2 Cancel: shared flag checked between chunks and before each tool call; turn ends as cancelled. Verify: fixture that pauses mid-stream ends within one second of the flag.
- [ ] 3.3 One-retry policy with backoff under two seconds. Verify: exactly two requests for a retryable failure, one for a terminal one.
- [ ] 3.4 Thinking control: disabled by default, per-turn enable, effort override, reasoning echoed on later requests. Verify: snapshot of request two shows prior reasoning; a test shows no `temperature` field when thinking is on.

## 4. Tools

- [x] 4.1 `bash`: `sh -c`, null stdin, 30 s timeout, process group, exit code, cancel via SIGTERM then SIGKILL. Verify: tests for timeout, a command reading stdin, and cancel.
- [x] 4.2 Spill: inline cap 4 KB, larger output to the session output directory, result carries head, tail, path, size. Verify: a 1 MB write produces a 4 KB result plus a readable spill file.
- [x] 4.3 `read` with `paths[]`, line numbers, 2,000-line default per file, explicit errors for missing, directory, non-UTF-8. Verify: three files in one call; each error path covered.
- [x] 4.4 `write`: directory creation, 5 MB refusal, unread-overwrite refusal. Verify: unit tests for each refusal and the happy path.
- [x] 4.5 `edit` with `edits[]` applied atomically, exact-single-match, unchanged-replacement rejection, failure naming the offending pair and its match count. Verify: five hunks succeed; a bad fourth hunk leaves the file unchanged.
- [x] 4.6 Path jail: canonicalize, resolve symlinks, reject `..` and outside-root paths. Verify: `../../etc/passwd`, an escaping symlink, and an allowed in-tree symlink.
- [x] 4.7 Permission classification: three modes, safe list, deny list, prompt on mutation outside the session directory, refusal returned as a tool result. Verify: `ls` runs unprompted, `npm install` prompts, `sudo` is refused without a prompt, and each mode is asserted.
- [x] 4.8 `search` and `fetch` tool wiring: five summaries, no page bodies, `fetch` capped and spilled, errors carrying the API reason. Verify: fixture tests for both operations.
- [x] 4.9 `ask` tool: question plus optional choices, blocking on the answer channel, choice and free-text results, dismissal result, more than six choices rejected. Verify: tests for each result shape and for the option cap.
- [x] 4.10 Jail behaviour per mode and the `yolo` startup warning. Verify: writes outside the directory prompt in `normal`, are refused after a no, and never prompt in `yolo`.
- [x] 4.10a Background jobs: `background` on bash, `job list/wait/kill`, output file per job, process groups, 55 s wait cap, completion system note, and no jobs after restart. Verify: a 3-second job is waited on and its output returned; a long job reports still-running; kill stops the group.
- [ ] 4.11 Privilege gating: refused unless `allow_sudo=true`, always prompts when enabled, deny list applies with or without the prefix. Verify: default refusal names the setting; an approved sudo command runs; `sudo rm -rf /` is refused.
- [ ] 4.12 Sudo terminal handoff: leave the alternate screen and raw mode, run attached to the tty, restore and redraw after. Verify: cached credentials skip the handoff; a password prompt works against a scratch sudoers rule; Ctrl+C at the prompt cancels cleanly and the screen is redrawn.

## 5. TUI

- [x] 5.1 Terminal setup: raw mode via termios, alternate screen, cursor visibility, panic hook, SIGINT/SIGTERM. Verify: forced panic in a debug build leaves a usable shell.
- [ ] 5.2 ANSI renderer: clear, wrap, cursor placement, colour escapes, no-colour fallback, full redraw within one frame budget. Verify: byte-level snapshot of one frame at 100x30.
- [ ] 5.2a Colour roles and palettes: six semantic roles, dark and light tables, truecolor and 256-colour variants, `NO_COLOR` and `color=never` parity. Verify: snapshot of the same frame in four colour modes, plus a no-colour test asserting zero escape bytes.
- [ ] 5.3 Streaming render with tail-follow, scroll detach, unseen-line count. Verify: snapshots after 100 appended tokens and after scrolling up.
- [ ] 5.4 Thinking line collapsed by default, expandable inline, token count. Verify: snapshots before and after expansion.
- [ ] 5.5 Tool lines: name, truncated arguments, duration, error colour, expand-last-output inline. Verify: snapshots for success and non-zero exit.
- [ ] 5.6 Status line: model, context estimate, cache hit share, cost, unknown-model fallback, one dim row only. Verify: unit tests over usage structs including a zero-hit response.
- [ ] 5.7 Key parsing from raw bytes: the full key set, including the double Ctrl+C guard. Verify: input-event tests per key.
- [ ] 5.8 Question picker: numbered choices, custom-answer row, typed answer echo, dismissal, and `y`/`n` shortcuts for a two-option yes/no question. Verify: snapshot and key-handling tests for pick, type, dismiss, and both y/n answers.
- [x] 5.9 Input history: append to the history file, Up/Down recall, consecutive de-duplication, 1,000-entry cap, survives restart. Verify: a scripted session sends three prompts, restarts, and recalls them.
- [x] 5.10 Slash commands: exactly six (`/help` `/new` `/sessions` `/compact` `/think` `/quit`), dispatch without a model request, unknown-command suggestion, and `/clear` correctly rejected. Verify: each command runs, `/clear` does nothing, and unknown input sends nothing.
- [x] 5.11 Slash autocomplete: menu above the input from the first `/`, prefix rank before substring, eight rows max with descriptions, Tab completes, Enter runs, Esc dismisses. Verify: key-event tests for `/co` narrowing, Tab, Enter, and Esc.
- [x] 5.12 `!` shell prefix: run through the same runner as the `bash` tool, output capped and spilled, exit code shown, no model request, denial still enforced, and a tail `system` record that leaves the prefix byte-identical. Verify: `!git status`, a non-zero command, a 1 MB output, `!sudo`, and a prefix-hash comparison before and after.

## 6. Context, prune, compaction

- [x] 6.1 Prefix assembly: built-in prompt, instructions, skill index, conversation. Frozen for the session. Verify: identical prefixes across two requests where cwd and usage changed.
- [ ] 6.2 Token estimator and thresholds. Verify: unit tests either side of 45k and 60k, and a test that the mid-loop case defers compaction.
- [ ] 6.3 Prune: older tool results collapse to a stub plus spill path at 45k, structure intact, no model call. Verify: a 70k history prunes below 60k and compaction does not run.
- [ ] 6.4 Compaction: four exchanges verbatim including reasoning and tool calls, summarizer with thinking disabled, one `system` message named `compacted`, before/after reporting. Verify: a 20-exchange history compacts to a summary plus four exchanges.
- [ ] 6.5 Drop orphan tool results after range removal. Verify: a result whose call was summarized is excluded.
- [ ] 6.6 Tail notes: memory writes and skill loads mid-session arrive as `system` messages without touching the prefix. Verify: prefix bytes identical either side of a memory save.
- [ ] 6.7 Mid-loop `context_length_exceeded`: truncate older tool results, retry once, then fail the turn. Verify: one retry is issued and the second failure is a clean error.

## 7. Memory

- [x] 7.1 Store: one `.md` per note, filename slug, title from the first heading or the filename, global and project directories, three-way project detection, re-read on every search. Verify: tests for `.git`, `.jj`, and `.mote`, for a bare directory, for a hand-written file, and for a file with no heading.
- [x] 7.2 `memory` tool with `search` and `save`; case-insensitive match over filename and body; up to five results with excerpts and paths; duplicate slug refused. Verify: search hit, no-hit, duplicate-slug, and external-edit tests.
- [x] 7.3 No prompt index: two-line pointer only. Verify: snapshot of the system prompt with eighty notes present contains no titles.
- [x] 7.4 Compaction proposes up to three durable facts as saves. Verify: a fixture conversation produces proposals and writes nothing.
- [ ] 7.5 Permission behaviour for memory writes in both scopes. Verify: a global save prompts in `normal`, a project save does not, and a refusal leaves the directory byte-identical.

## 8. Skills and agents

- [x] 8.1 Skill discovery from global, `.mote/`, and extra roots, project shadowing, malformed file skipped once with a report. Verify: tests for a duplicate name and bad frontmatter.
- [ ] 8.2 Skill index in the prompt, one line each, capped at 40. Verify: twenty skills add twenty short lines.
- [x] 8.3 `skill_load` with a 4 KB cap. Verify: oversized body truncated with a marker.
- [x] 8.4 Agent loading: `agents/<name>.md`, body as instructions, `tools`/`model`/`thinking` frontmatter, built-in `default`, three roots with project shadowing. Verify: a restricted agent has no write tool, and `permissions:` in an agent file is ignored.
- [ ] 8.5 `--agent` at launch and `/agent` picker mid-conversation, keeping the transcript. Verify: switching changes the tool list and instructions, and switching back restores both.
- [ ] 8.6 Switch delivery as a tail `system` note that supersedes prior instructions, with no prefix rewrite and a reported no-reset. Verify: prefix hash identical either side of a switch.
- [ ] 8.7 Record the switch in the session log and restore the active agent on resume. Verify: round-trip a session with one switch.

## 8b. Setup

- [x] 8b.1 `mote init`: create config and state directories, print `created`/`kept` per path, overwrite nothing. Verify: two consecutive runs create then keep, asserted in a temp HOME.
- [x] 8b.2 Two examples only: `agents/example.md` that loads as a valid restricted agent and `skills/git/SKILL.md` with a routing description; commented config template with no key. Verify: both parse, template has no `api_key`, and a test asserts the file count.
- [x] 8b.3 Refuse agent writes to the main config file in `normal` and `strict`, prompt in `yolo`, and record the refusal. Verify: `write` on the config path is refused with a message naming the user, and a skill write still succeeds.
- [x] 8b.4 Built-in `config` skill compiled into the binary, describing the layout, formats, and the no-self-edit rule. Verify: it appears in the skill index without any file on disk, and its body loads.

## 9. Instructions

- [x] 9.1 Discovery of global, repository-root, and session-directory `AGENTS.md`, with `CLAUDE.md` as a per-directory fallback only. Verify: tests for all three roots, the precedence order, and both names present.
- [x] 9.2 8 KB cap with explicit truncation, and a startup report listing files and sizes. Verify: a 40 KB file yields an 8 KB prompt section and a reported truncation.
- [x] 9.3 Explicit reload that reports the cache reset. Verify: editing the file mid-session has no effect until reload, and reload changes the prefix exactly once.

- [x] 9.4 Compiled baseline prompt: tool semantics, caps, confinement, ask policy; agent files add role only. Verify: every tool name appears, both caps appear, and the size test passes.
- [x] 9.5 `mote prompt` prints the baseline alone. Verify: output contains no instructions, skills, or memory text.

## 10. Sessions

- [x] 10.1 JSONL writer with flush after each record, covering messages, tool calls, results, compaction, and usage. Verify: kill during a tool call leaves every prior record readable.
- [x] 10.2 Sidecar per session with title, cwd, model, counts, totals. Verify: listing one thousand sidecars reads no transcripts.
- [x] 10.3 Title from the first user message, rename touching only the sidecar. Verify: transcript mtime and bytes unchanged after a rename.
- [ ] 10.4 `-c`, `-r <id>`, explicit path, and the in-app picker with newest first and a "new session" entry. Verify: each entry point loads the intended file; the picker test uses a fixture directory.
- [ ] 10.4a CLI management: `--list` prints id, title, age, and message count then exits; `--rename <id> <title>`; `--delete <id>` with confirmation. Verify: a fixture directory with three sessions, asserting output columns, exit status, and that transcripts are byte-identical after a rename.
- [ ] 10.4b Picker actions: Enter opens, `n` starts new, `r` renames, `d` deletes after confirmation. Verify: key-event tests for each action against a fixture directory.
- [x] 10.5 Delete moves both files to trash after confirmation. Verify: files absent from the list and present in trash.
- [x] 10.6 Wire-fidelity resume: reasoning, tool calls, matching results, summary; orphans dropped; append to the same file. Verify: round-trip that logs a turn and rebuilds an identical request body.
- [ ] 10.7 Subagents create no session entries. Verify: ten delegations add zero rows to the list.

## 11. Polish and proof

- [ ] 11.1 Recorded-fixture replay harness so the whole loop runs offline. Verify: `cargo test` passes with networking disabled.
- [ ] 11.2 Scripted tmux run: three prompts, assert a tool line and a final answer. Verify: script exits zero on a live key and prints the captured screen on failure.
- [ ] 11.3 Measure and record: binary size, RSS, cold start, cache hit share over ten turns. Verify: numbers in `README.md` compared with the budget.
- [ ] 11.4 Mode selection: `--cli`, `--chat`, and the tty-based default, with the line mode emitting no escape sequences. Verify: a scripted run with stdout piped contains zero ESC bytes, and `--chat` in a terminal opens the alternate screen.
- [ ] 11.5 Ship a starter skill set for common CLIs (git, rg, jq, gh, docker, macOS) with routing descriptions, reusing the existing llm-functions skill files where they fit. Verify: each skill loads, and a request naming the tool routes to it.
- [x] 11.6 `README.md`: install, key, keys, config, memory, skills, agents, instructions, sessions. Verify: a fresh clone reaches a first answer following only the README.

## Workflow follow-up

- Run `/openspec-apply-change add-mote-agent` to execute these tasks.
- After acceptance, `/openspec-archive-change add-mote-agent` merges the nine capability specs into `openspec/specs/`.

## 12. Close the spec gaps

- [x] 12.1 `yolo` disables the path jail; other modes keep it. Verify: a write outside the directory succeeds in `yolo` and is refused in `normal`.
- [x] 12.2 Terminal restored on panic and on SIGTERM, not only on a normal exit. Verify: a forced panic and a SIGTERM both leave the terminal usable.
- [x] 12.3 `bash` cannot rewrite the main config either: the command text is checked for the config path. Verify: `echo x > ~/.config/mote/config` is refused in `normal` and prompts in `yolo`.
- [ ] 12.4 Mid-loop `context_length_exceeded`: truncate old tool results, retry once, then fail cleanly. Verify: a forced 400 with that body produces one retry and a second failure ends the turn.
- [x] 12.5 Agent switches are logged and restored on resume. Verify: switch, resume, and the same agent is active.
- [x] 12.6 Compaction proposes up to three memory saves and writes only what is approved. Verify: a fixture summary with `MEMORY:` lines produces proposals.
- [ ] 12.7 `/sessions` loads the chosen session in place. Verify: choosing an entry replaces the transcript and continues that session.
- [x] 12.8 `memory` and `skill_load` appear only when configured. Verify: with no memory directory and only built-in skills, the schema has neither.
- [x] 12.9 `/reload` re-reads instructions and skills, rebuilds the prefix, and reports the cache reset. Verify: editing `AGENTS.md` changes the next request only after reload.

## 13. Process

- [x] 13.1 Delete `scripts/check.sh`; document the three commands it wrapped in `AGENTS.md` and `README.md`. Verify: `cargo test`, `openspec validate --all --strict`, and `scripts/size.sh` run standalone.
