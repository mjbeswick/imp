# Tasks

## 1. Background events

- [x] 1.1 A background pane's finished turn, failure, question or confirmation sets a toast `{n}/{m} {name}: {note}` after the swap back; the current session's events set no toast; tests
- [x] 1.2 A background wake and a background job notice set the toast; the newest toast replaces the older; tests

## 2. Job notices and line mode

- [x] 2.1 Job completions are emitted as `Ui::Notice`, shown in the transcript as before and in line mode as a plain line; tests
