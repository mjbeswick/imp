# Tasks

- [x] 1.1 Wait 60 seconds for an answer and stop the turn when none arrives. Verify: a unit test with a 50 ms deadline reports no answer, and a stop event sets the cancel flag.
- [x] 1.2 Show the seconds left in the confirmation row. Verify: a frame test shows the deadline and a second frame shows a smaller number.
- [x] 1.3 Ring the bell once when a prompt opens. Verify: a frame test sees `\u{7}` while the prompt is fresh and not once it is stale.
- [x] 1.4 Refuse and stop the turn on Ctrl+C while a prompt is open. Verify: a key test clears the prompt and sets the cancel flag.
- [x] 1.6 Clear the open prompt when a stop arrives. Verify: a stop event leaves no pending prompt and no countdown.
- [ ] 1.7 Stop the worker before it can send another request after a timeout. Verify: an end-to-end run shows the stop line, no stale prompt, and no further model output.
- [ ] 1.5 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
