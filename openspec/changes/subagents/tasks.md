# Tasks: subagents

## 1. Profile lookup and tool schema

- [ ] 1.1 Test: `task` is in the schema, is offered at depth 0, and refuses a missing or unknown `agent` with the list of names. Expected: the refusal lists `build`, `plan`, `review`, and any roster profile.
- [ ] 1.2 Add the `task` schema entry and the lookup against `self.roster`.

## 2. Child run and state restore

- [ ] 2.1 Test: a child run with a scripted reply returns the child's final text as the tool result and restores history, profile, and model on success.
- [ ] 2.2 Test: a child that fails with a transport error leaves the parent history, profile, and model unchanged.
- [ ] 2.3 Implement the child run with a single state-restore guard covering history, profile, model, thinking, effort, and the child flag.
- [ ] 2.4 Test: the child's request offers only the profile's tools, and `task` is absent.

## 3. Depth, logging, and cancellation

- [ ] 3.1 Test: a child profile that lists `task` is not offered it, and a `task` call during a child is refused.
- [ ] 3.2 Test: each child message is written as a `t:"child"` record tagged with the `task` call id; no new session file exists; the parent's `task` call and result are logged as `t:"msg"`.
- [ ] 3.2a Test: resuming a session with nested records sends only parent messages in the next request.
- [ ] 3.3 Test: the child stops at `max_steps` and says so.
- [ ] 3.4 Test: cancelling during a child ends the turn with the cancelled notice.
- [ ] 3.5 Implement the depth flag, log guard, step cap, and cancellation check.

## 4. Permissions and transcript

- [ ] 4.1 Test: a child's confirmation goes to the parent's `confirm` with the child's name in the prompt, and a declined answer refuses the child's write.
- [ ] 4.2 Test: a child's `Content` and `Reasoning` are not forwarded to the parent; its `ToolStart` carries the child name.
- [ ] 4.3 Implement `ChildHooks`.

## 5. Drill-in inspection

- [ ] 5.1 Test: opening the `task` row of a finished child lists its messages, tool calls with arguments and results, and final text in order.
- [ ] 5.2 Test: Esc returns to the parent and leaves both histories unchanged.
- [ ] 5.3 Test: after resume, the child transcript rebuilt from nested records matches the one shown before.
- [ ] 5.4 Test: a running child's view shows messages so far and gains new ones.
- [ ] 5.5 Implement the child-transcript map, the nested record writer and reader, and the read-only view with its key binding in `src/tui.rs`.

## 6. Documentation and validation

- [ ] 6.1 Document the `task` tool and the drill-in key in `README.md` and `docs/agent-loop.md`.
- [ ] 6.2 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Record the measured size in the archive commit.
- [ ] 6.3 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
