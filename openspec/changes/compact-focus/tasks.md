# Tasks

Scenarios map to tests as follows. Conversation-context: focus added, no focus, long focus, recent
exchanges kept, automatic compaction. TUI: compact with focus, compact alone, no picker for focus
text, help line.

## 1. Tests first (red)

- [ ] 1.1 Focus line: `Session::compact` with `Some("keep the auth decisions")` puts `Focus: keep the auth decisions` in the summary request before the transcript. Verify: agent unit test on the captured request, failing before the change.
- [ ] 1.2 No focus: `Session::compact` with `None` sends a request with no `Focus:` line, and the request is byte-identical to the pre-change prompt. Verify: agent unit test comparing against the fixed prompt.
- [ ] 1.3 Long and blank focus: text is trimmed, cut at 500 characters, and the reply says `focus cut to 500 characters`; whitespace-only text counts as no focus. Verify: agent unit tests with 600 characters and with `"   "`.
- [ ] 1.4 Recent exchanges kept: a focused compaction keeps the last four exchanges verbatim after the summary, with the 400-token cap unchanged. Verify: agent unit test on the resulting transcript.
- [ ] 1.5 Automatic compaction: the token-budget path calls compaction with no focus. Verify: agent test that triggers compaction past `compact_at` and checks no `Focus:` line.
- [ ] 1.6 Compact with focus: `/compact keep the migration plan` sends `Cmd::Compact` with that focus, clears the composer, and opens no picker. Verify: tui test on the command arm.
- [ ] 1.7 Compact alone: `/compact` sends `Cmd::Compact` with no focus, as before. Verify: tui test on the existing `/compact` test.
- [ ] 1.8 No picker for focus text: typing `/compact keep` leaves the text in the composer and opens no picker. Verify: tui test on the composer state.
- [ ] 1.9 Help line: the `/compact` help row reads `/compact [focus]`. Verify: help test over the row text.

## 2. Implementation

- [ ] 2.1 `Session::compact` takes `Option<&str>` focus; trims, cuts at 500 characters, adds the `Focus:` line before the transcript, and reports the cut. Verify: tasks 1.1, 1.2, 1.3, 1.4 pass.
- [ ] 2.2 Automatic compaction passes `None`. Verify: task 1.5 passes.
- [ ] 2.3 `Cmd::Compact` carries the focus; the `/compact` arm parses text after the word; the worker passes it on. Verify: tasks 1.6, 1.7, 1.8 pass.
- [ ] 2.4 Help line reads `/compact [focus]`. Verify: task 1.9 passes.

## 3. Existing tests and validation

- [ ] 3.1 Update existing compaction tests in `src/agent.rs` and `src/tui/tests.rs` to the new signature. Verify: `cargo test` runs in full and passes.
- [ ] 3.2 `openspec validate compact-focus --strict` and `openspec validate --all --strict` pass with no warnings. Verify: both commands run and report valid.
- [ ] 3.3 `./scripts/size.sh` reports the new size. Verify: the command runs and prints the size line; nothing is recorded until archive.

## 4. Reinstall

- [ ] 4.1 Reinstall the binary to `~/.local/bin/imp` after the merge. Verify: `imp --version` reports the new build. Left unticked: the lead installs after the merge, not this apply.

## 5. Real-terminal check

- [ ] 5.1 In a real terminal, run `/compact keep the open questions` on a session with history, and confirm the summary reflects the focus and the composer clears. Verify: a manual run recorded in the archive note. Left unticked: needs a person at a terminal.
