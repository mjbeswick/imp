# Tasks

## 1. Listing

- [x] 1.1 `Log::listing` includes a session with metadata and no transcript; `list` and `latest` are unchanged; tests
- [x] 1.2 The sessions modal and search use `listing`; a blank session shows as `(untitled)`; tests

## 2. Marks

- [x] 2.1 `Ev::Done(Some(_))` sets the pane's failed flag and `Ev::Done(None)` clears it; the flag survives a pane switch; tests
- [x] 2.2 A row shows `?` while waiting, the spinner while running, `!` when failed, and `•` when unseen, in that priority; idle rows have no mark; the marks appear in search too; tests

## 3. Docs

- [x] 3.1 README's sessions section lists the marks
