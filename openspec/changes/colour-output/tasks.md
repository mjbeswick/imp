# Tasks

- [ ] 1.1 Keep only SGR in shown output, measure and cut it by visible width, and paint raw rows. Verify: tests that cursor moves and OSC are dropped, colours stay, and a cut row keeps its codes and ends reset.
- [ ] 1.2 Show typed `!` output in colour in both front-ends. Verify: a test that coloured output keeps its SGR in the chat and is plain with colour off.
- [ ] 1.3 Add `show` to `bash`. Verify: tests that the user sees the coloured output and the model gets plain text or a picture placeholder.
- [ ] 1.4 Render closed fenced blocks through `render.<lang>` commands, cached, with a two-second limit. Verify: tests for a rendered block, a failing command showing code, an open fence left alone, and one run for repeated draws.
- [ ] 1.5 Update the prompt and `README.md`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
