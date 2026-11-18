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

## 5. TUI

- [x] 5.1 Terminal setup: raw mode via termios, alternate screen, cursor visibility, panic hook, SIGINT/SIGTERM. Verify: forced panic in a debug build leaves a usable shell.
- [x] 5.9 Input history: append to the history file, Up/Down recall, consecutive de-duplication, 1,000-entry cap, survives restart. Verify: a scripted session sends three prompts, restarts, and recalls them.
- [x] 5.10 Slash commands: exactly six (`/help` `/new` `/sessions` `/compact` `/think` `/quit`), dispatch without a model request, unknown-command suggestion, and `/clear` correctly rejected. Verify: each command runs, `/clear` does nothing, and unknown input sends nothing.
- [x] 5.11 Slash autocomplete: menu above the input from the first `/`, prefix rank before substring, eight rows max with descriptions, Tab completes, Enter runs, Esc dismisses. Verify: key-event tests for `/co` narrowing, Tab, Enter, and Esc.
- [x] 5.12 `!` shell prefix: run through the same runner as the `bash` tool, output capped and spilled, exit code shown, no model request, denial still enforced, and a tail `system` record that leaves the prefix byte-identical. Verify: `!git status`, a non-zero command, a 1 MB output, `!sudo`, and a prefix-hash comparison before and after.

## 6. Context, prune, compaction

- [x] 6.1 Prefix assembly: built-in prompt, instructions, skill index, conversation. Frozen for the session. Verify: identical prefixes across two requests where cwd and usage changed.

## 7. Memory

- [x] 7.1 Store: one `.md` per note, filename slug, title from the first heading or the filename, global and project directories, three-way project detection, re-read on every search. Verify: tests for `.git`, `.jj`, and `.mote`, for a bare directory, for a hand-written file, and for a file with no heading.
- [x] 7.2 `memory` tool with `search` and `save`; case-insensitive match over filename and body; up to five results with excerpts and paths; duplicate slug refused. Verify: search hit, no-hit, duplicate-slug, and external-edit tests.
- [x] 7.3 No prompt index: two-line pointer only. Verify: snapshot of the system prompt with eighty notes present contains no titles.
- [x] 7.4 Compaction proposes up to three durable facts as saves. Verify: a fixture conversation produces proposals and writes nothing.

## 8. Skills and agents

- [x] 8.1 Skill discovery from global, `.mote/`, and extra roots, project shadowing, malformed file skipped once with a report. Verify: tests for a duplicate name and bad frontmatter.
- [x] 8.3 `skill_load` with a 4 KB cap. Verify: oversized body truncated with a marker.
- [x] 8.4 Agent loading: `agents/<name>.md`, body as instructions, `tools`/`model`/`thinking` frontmatter, built-in `default`, three roots with project shadowing. Verify: a restricted agent has no write tool, and `permissions:` in an agent file is ignored.

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
- [x] 10.5 Delete moves both files to trash after confirmation. Verify: files absent from the list and present in trash.
- [x] 10.6 Wire-fidelity resume: reasoning, tool calls, matching results, summary; orphans dropped; append to the same file. Verify: round-trip that logs a turn and rebuilds an identical request body.

## 11. Polish and proof

- [x] 11.6 `README.md`: install, key, keys, config, memory, skills, agents, instructions, sessions. Verify: a fresh clone reaches a first answer following only the README.

## Workflow follow-up

- Run `/openspec-apply-change add-mote-agent` to execute these tasks.
- After acceptance, `/openspec-archive-change add-mote-agent` merges the nine capability specs into `openspec/specs/`.

## 12. Close the spec gaps

- [x] 12.1 `yolo` disables the path jail; other modes keep it. Verify: a write outside the directory succeeds in `yolo` and is refused in `normal`.
- [x] 12.2 Terminal restored on panic and on SIGTERM, not only on a normal exit. Verify: a forced panic and a SIGTERM both leave the terminal usable.
- [x] 12.3 `bash` cannot rewrite the main config either: the command text is checked for the config path. Verify: `echo x > ~/.config/mote/config` is refused in `normal` and prompts in `yolo`.
- [x] 12.5 Agent switches are logged and restored on resume. Verify: switch, resume, and the same agent is active.
- [x] 12.6 Compaction proposes up to three memory saves and writes only what is approved. Verify: a fixture summary with `MEMORY:` lines produces proposals.
- [x] 12.8 `memory` and `skill_load` appear only when configured. Verify: with no memory directory and only built-in skills, the schema has neither.
- [x] 12.9 `/reload` re-reads instructions and skills, rebuilds the prefix, and reports the cache reset. Verify: editing `AGENTS.md` changes the next request only after reload.

## 13. Process

- [x] 13.1 Delete `scripts/check.sh`; document the three commands it wrapped in `AGENTS.md` and `README.md`. Verify: `cargo test`, `openspec validate --all --strict`, and `scripts/size.sh` run standalone.
