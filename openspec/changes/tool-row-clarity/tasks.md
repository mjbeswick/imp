# Tasks

## 1. Finishes

- [x] 1.1 The shared finish; `!`, muted and red `✗` in the TUI; tests for each finish
- [x] 1.2 Exited rows show the last three output lines, line breaks kept; tests for a short and a long tail at 54 columns
- [x] 1.3 Line mode prints the same four finishes; test

## 2. Expansion

- [x] 2.1 Keep the latest turn's rows; Ctrl+E toggles all of them; tests with a `✗` before a `✓`
- [x] 2.2 A click toggles one row; test
- [x] 2.3 `$ command` and `key: value` lines instead of JSON; tests for bash and `edit`
- [x] 2.4 A picker over the tool rows takes the click, not the row beneath; test
- [x] 2.5 Cap expanded arguments at 20 lines, as output is; test with a 50-line `write`

## 3. Declines

- [x] 3.1 A decline shows once, as `declined` on its row; test
- [x] 3.2 No suggestion after a declined turn; test
