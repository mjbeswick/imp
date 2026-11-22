# Tasks

## 1. Actions on the open message

- [x] 1.1 `Ctrl+Q` sends the open message as a steer or a new turn and removes it; the rule says `steer now` or `send now`; tests for both states
- [x] 1.2 `Ctrl+X` deletes and opens the next entry; `Ctrl+C` closes unchanged and does not quit; tests for delete, delete the last, and close

## 2. Releasing the queue

- [x] 2.1 Every editor exit releases the queue when the turn ended mid-edit, and a stale index cannot panic; tests for ↑ into history and a queue that shrank
