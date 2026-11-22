# Tasks

## 1. Wait until answered

- [x] 1.1 Remove the deadline, the expiry, and the countdown; tests that an old prompt stays open, Enter approves, and the row has no seconds
- [x] 1.2 The loop sleeps as when idle while a prompt waits; test
- [x] 1.3 Esc on a confirmation or question declines and stops the turn; `n` still declines one step; tests for both
- [x] 1.4 The worker waits for a question's answer with no limit (it gave up after 600 s); a test that a late answer still arrives
