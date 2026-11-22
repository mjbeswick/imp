# tools Specification

## Purpose
Defines the five built-in tools — shell, read, edit, write, search — their batching, their output caps, the path jail, and the confirmation gate. Owns what the model can do and what it costs in prompt space.

## Requirements

### Requirement: The tool set is small and shell-first

The system SHALL expose `bash`, `read`, `edit`, `write`, `job`, `ask`, and `fetch` as its core tool set, SHALL expose `search` only when the active provider implements the search endpoint, SHALL NOT expose per-CLI tools such as `gh`, `docker`, `kubectl`, `git`, glob, grep, list, patch, or todo, and SHALL teach specific command-line tools through skills instead.

#### Scenario: Default prompt
- **WHEN** a session starts with memory and skills configured
- **THEN** the request carries the seven core tools plus `memory` and `skill_load`, and no per-CLI tool

#### Scenario: A CLI is not installed as a tool
- **WHEN** the model needs `gh`
- **THEN** it loads the `gh` skill if one is installed and runs `gh` through `bash`, rather than calling a `gh` tool

#### Scenario: File discovery without a glob tool
- **WHEN** the model needs to find files by pattern
- **THEN** it uses `bash` and the task completes without a dedicated tool

#### Scenario: No search capability
- **WHEN** the provider has no search endpoint
- **THEN** the tool list omits `search`, keeps `fetch`, and the prompt has no search line

### Requirement: Asking a yes/no question is one keystroke

The system SHALL render a question whose only choices are yes and no as a `y`/`n` prompt, SHALL return the chosen word, and SHALL treat `y` and `n` as typed text on any other question.

#### Scenario: Confirmation-style question
- **WHEN** the model asks a question with options `["Yes", "No"]`
- **THEN** pressing `y` answers Yes and pressing `n` answers No

#### Scenario: Other two-choice question
- **WHEN** the options are `["Python", "Rust"]` and the user presses `y`
- **THEN** nothing is sent and the answer row reads `y`

#### Scenario: Escape
- **WHEN** the user presses Esc on any question
- **THEN** the result says the question was not answered

### Requirement: The agent can ask the user a question

The system SHALL provide an `ask` tool taking a question, optional two-to-six choices each with a label and an optional description, and an optional `multiple` flag, SHALL block until the user answers, SHALL return the chosen label, the marked labels joined by `, ` under `multiple`, or the user's typed answer, and SHALL stop the turn when the user dismisses the question.

#### Scenario: Multiple choice
- **WHEN** the model asks which of three deployment targets to use
- **THEN** the user picks one and the model receives that exact text as the tool result

#### Scenario: Custom answer
- **WHEN** the user ignores the offered choices and types their own answer
- **THEN** the model receives the typed text

#### Scenario: No options
- **WHEN** the model asks a question with no choices
- **THEN** the interface shows a free-text prompt and returns what the user types

#### Scenario: Declined
- **WHEN** the user dismisses the question
- **THEN** the tool result says the user did not answer and the turn stops

#### Scenario: Several choices
- **WHEN** the model asks with `multiple: true` and the user marks the first and third options
- **THEN** the model receives both labels joined by `, `

#### Scenario: Described option
- **WHEN** an option has a description
- **THEN** the user sees it under the label and the model receives only the label

#### Scenario: Too many options
- **WHEN** the model supplies more than six choices
- **THEN** the call is rejected with a message asking for fewer

### Requirement: Shell commands run non-interactively

The system SHALL run `bash` commands through `sh -c` with stdin on the null device, a 30 second timeout, and a process group, SHALL return the exit code, and SHALL hand a model call still running at the timeout to the job table instead of terminating it.

#### Scenario: Prompting command
- **WHEN** a command would wait for terminal input
- **THEN** it reads end of file immediately instead of hanging the agent

#### Scenario: Timeout
- **WHEN** a model command runs longer than 30 seconds
- **THEN** it keeps running as a background job and the result names the job id, its log, and `job wait`

#### Scenario: Typed timeout
- **WHEN** a typed `!` command runs longer than 30 seconds
- **THEN** the process group is terminated and the result reports a timeout

#### Scenario: Cancel
- **WHEN** the user cancels while a command runs
- **THEN** the process group gets SIGTERM, then SIGKILL after two seconds, and the result reads cancelled

### Requirement: Long commands run as background jobs

The system SHALL start a `bash` command as a job when `background` or `delay_s` is set, SHALL start it after `delay_s` seconds when given, SHALL return a job id and its output path immediately, SHALL cap `job wait` at 55 seconds and return partial output, SHALL show each job's elapsed time, output size, and last output line, and SHALL report a finished job to the model as a `system` note at the tail.

#### Scenario: Start and continue
- **WHEN** the model starts `make` in the background
- **THEN** it gets a job id at once and the turn can continue

#### Scenario: Wait with a budget
- **WHEN** the model waits on a running job
- **THEN** it receives output produced so far and either an exit code or "still running"

#### Scenario: Completion note
- **WHEN** a background job exits between turns
- **THEN** a system note records the job id, exit code, and output path

#### Scenario: Kill
- **WHEN** the model kills a job
- **THEN** its process group receives SIGTERM then SIGKILL

#### Scenario: Jobs do not survive a restart
- **WHEN** the process exits with a job running
- **THEN** no record of that job exists in the next session

#### Scenario: Delayed start
- **WHEN** the model runs `rsync -a src/ dst/` with `delay_s` 3600
- **THEN** the job is listed as starting in an hour and nothing runs until then

#### Scenario: Cancelled before it starts
- **WHEN** the model kills a delayed job before its delay ends
- **THEN** the command never runs

#### Scenario: Progress
- **WHEN** a running job has printed `1,234,567  45%  12.0MB/s`
- **THEN** `job list` shows its elapsed time, output size, and that line

### Requirement: Large results are buffered

The system SHALL return at most 4 KB inline from a command, job wait, or fetch result, SHALL keep the full output in a buffer addressed as `imp://out/<n>`, SHALL hold buffers in memory up to 16 MB, SHALL demote the least-recently-used buffer to an unlinked temporary file when memory is full, SHALL cap unlinked spill at 64 MB and drop the oldest file-backed buffer past it, SHALL include the address, the byte count, and a hint to read a range, and SHALL report an expired buffer by name.

#### Scenario: Big command output
- **WHEN** a command writes 1 MB to stdout
- **THEN** the result holds a short head and tail, an `imp://out/<n>` address, and the original size

#### Scenario: Read the buffer
- **WHEN** the model reads `imp://out/3` with offset and limit
- **THEN** it receives that slice with line numbers and no filesystem access

#### Scenario: Demotion keeps the id
- **WHEN** memory is full and the oldest buffer is demoted to a temporary file
- **THEN** reading its address still returns the text and no file appears in any directory

#### Scenario: Spill cap
- **WHEN** file-backed buffers exceed 64 MB
- **THEN** the oldest is dropped and its address reports expiry

#### Scenario: Expired buffer
- **WHEN** an address was dropped
- **THEN** the result says the buffer expired and names the address

#### Scenario: New session
- **WHEN** the user runs `/new`
- **THEN** the new session starts with no buffers, and the previous session keeps its own while it stays live

### Requirement: Job logs stay readable

The system SHALL write background job output to a file under the output directory, SHALL allow `read` to open files inside that directory, and SHALL delete job log files older than seven days when a session starts.

#### Scenario: Job log readable
- **WHEN** the model reads a job log path from a previous result
- **THEN** `read` returns the content instead of refusing it as outside the session

#### Scenario: Old log removed
- **WHEN** a session starts and a job log is eight days old
- **THEN** the file is deleted

#### Scenario: Recent log kept
- **WHEN** a session starts and a job log is one day old
- **THEN** the file stays

### Requirement: Long reads are trimmed

The system SHALL NOT buffer a `read` result: a read longer than 4 KB SHALL return a head and a tail with a hint to use offset and limit, and SHALL create no buffer.

#### Scenario: Read does not buffer
- **WHEN** a `read` result exceeds 4 KB
- **THEN** the result holds a head and tail with a hint to use offset and limit, and no buffer is created

### Requirement: Reads are batched and numbered

The system SHALL accept one or more paths in a single `read` call, SHALL return each file with line numbers and default to 2,000 lines per file, SHALL list a directory as a sorted tree with directories marked, SHALL default to one level of depth and accept at most three, SHALL cap each directory at 200 entries and say when it cut the list, SHALL skip `.git`, `.jj`, `target`, `node_modules`, `dist`, `build`, and `.venv`, and SHALL state clearly when a path is missing or not valid UTF-8.

#### Scenario: Three files at once
- **WHEN** the model reads three paths in one call
- **THEN** all three appear in one result and no second call is needed

#### Scenario: Directory listing
- **WHEN** the model reads `src/`
- **THEN** the result lists its entries in name order, each directory marked with a trailing `/`, and no file contents

#### Scenario: Depth
- **WHEN** the model reads `src/` with `depth: 2`
- **THEN** immediate entries and their children appear, nested by one indent per level

#### Scenario: Default depth
- **WHEN** the model reads a directory without `depth`
- **THEN** only its immediate entries appear

#### Scenario: Noise skipped
- **WHEN** the directory contains `.git`, `target`, and `node_modules`
- **THEN** none of them appear in the listing

#### Scenario: Mixed call
- **WHEN** the model reads a directory and a file in one call
- **THEN** the listing and the numbered file both appear

#### Scenario: Long directory
- **WHEN** a directory holds 500 entries
- **THEN** the first 200 appear in name order with a note that the rest were cut

#### Scenario: Large file
- **WHEN** a file has 10,000 lines and no range is given
- **THEN** the first 2,000 lines are returned with a note on how to continue

#### Scenario: Binary file
- **WHEN** the path is not valid UTF-8
- **THEN** the result says so and does not include mojibake

### Requirement: Writes are deliberate

The system SHALL create parent directories when writing, SHALL refuse files larger than 5 MB, and SHALL NOT overwrite a non-empty file that has not been read during the current session.

#### Scenario: Blind overwrite refused
- **WHEN** the model writes to an existing file it has not read
- **THEN** the write is refused and the result tells the model to read it first

#### Scenario: New file
- **WHEN** the model writes a path that does not exist
- **THEN** the directories are created and the file is written

### Requirement: Edits are batched and atomic

The system SHALL accept a list of old/new pairs in one `edit` call, SHALL apply all of them or none, and SHALL refuse the call when any old string matches other than exactly once or the replacement is unchanged.

#### Scenario: Five hunks, one call
- **WHEN** the model sends five old/new pairs for one file
- **THEN** all five are applied in one call and one round trip

#### Scenario: One bad hunk
- **WHEN** the fourth pair matches three times
- **THEN** nothing is written and the result reports which pair failed and how many matches it found

#### Scenario: Unchanged replacement
- **WHEN** a pair's old and new strings are identical
- **THEN** the call is rejected without writing

### Requirement: Paths stay inside the session directory

The system SHALL resolve every path, including symlinks and `..`, and SHALL reject any that leaves the session's start directory unless the user disabled the jail at launch.

#### Scenario: Escape attempt
- **WHEN** a tool is given `../../etc/passwd`
- **THEN** the call is rejected and no filesystem access occurs

#### Scenario: Symlink escape
- **WHEN** a path inside the directory resolves through a symlink to outside it
- **THEN** the call is rejected

### Requirement: Tools respect the permission decision

The system SHALL classify every tool action before executing it against the live permission mode, SHALL refuse actions the policy denies, SHALL wait for the user on actions the policy asks about, and SHALL return the refusal to the model as a result rather than an error.

#### Scenario: Denied by policy
- **WHEN** a command matches the deny list
- **THEN** it is not run and the model is told it was refused

#### Scenario: Asked and approved
- **WHEN** an action requires confirmation and the user approves
- **THEN** it runs and the result is returned normally

#### Scenario: Asked and refused
- **WHEN** the user refuses
- **THEN** nothing happens and the model receives a result saying the user declined

#### Scenario: Mode changes mid-turn
- **WHEN** the user enables yolo after the turn started
- **THEN** the turn's next tool call does not ask for confirmation

### Requirement: Search returns summaries, not pages

The system SHALL provide a `search` tool, on providers that implement it, that queries the web and returns at most five results containing title, URL, summary, date, and a short snippet, and SHALL NOT fetch or return page bodies.

#### Scenario: Web question
- **WHEN** the model searches for a current fact
- **THEN** it receives summaries with URLs in one round trip and no page content

#### Scenario: No results
- **WHEN** a search returns an empty result list
- **THEN** the result says so plainly and is reported as unbilled

### Requirement: Fetch returns page text as markdown

The system SHALL provide a local `fetch` tool for every provider that makes one `http` or `https` GET with a 15 second limit and returns the status line and body, SHALL return JSON and text unchanged, HTML as markdown with absolute links and without scripts, menus, or footers, and RSS and Atom as a list of title, date, and link, and SHALL apply the 4 KB cap and spill rule.

#### Scenario: Reading an article
- **WHEN** the model fetches an HTML page
- **THEN** it receives the status line, the page title as a heading, and the text as markdown without scripts, tags, or navigation

#### Scenario: A markdown file
- **WHEN** the model fetches a `.md` file with headings
- **THEN** the result is the file's text as served, beginning with its first line

#### Scenario: A JSON API
- **WHEN** the model fetches `https://api.open-meteo.com/v1/forecast?...`
- **THEN** it receives `200 application/json` and the JSON body unchanged

#### Scenario: Local network
- **WHEN** the model fetches `http://192.168.0.61:11080/`
- **THEN** the request is made from this machine and the device answers

#### Scenario: Other schemes
- **WHEN** the model fetches `file:///etc/passwd`
- **THEN** it is refused and no request is made

#### Scenario: A feed
- **WHEN** the model fetches `https://feeds.bbci.co.uk/news/rss.xml`
- **THEN** it receives one line per item with its title, date, and link, and no XML

#### Scenario: Links
- **WHEN** a page links `/news/123` with the text `Budget`
- **THEN** the markdown holds `[Budget](https://host/news/123)`

### Requirement: Search failures are bounded

The system SHALL retry a search or fetch once on 429, 408, or 5xx, SHALL report other errors without retrying, and SHALL never block a turn on a web failure.

#### Scenario: Rate limited
- **WHEN** the search endpoint returns 429
- **THEN** one retry follows after a short delay, and a second failure becomes a tool error

#### Scenario: Page unavailable
- **WHEN** fetch returns 404 or 403
- **THEN** the model gets an error naming the reason and the turn continues

### Requirement: Shell commands run in the session root

The system SHALL start every `bash` command in the session root, SHALL take that root from `-C, --cwd <PATH>` when given, from `/cd` when entered, and from the launch directory otherwise, SHALL resolve a relative `--cwd` against the launch directory and a relative `/cd` against the current root, SHALL refuse a missing or non-directory value before any request is sent, and SHALL move the path jail and typed `!` commands to the same directory.

#### Scenario: Default root
- **WHEN** the user runs `imp` from a project directory and the model runs `pwd`
- **THEN** the command prints that project directory

#### Scenario: Chosen root
- **WHEN** the user runs `imp --cwd ~/Projects/imp` and the model runs `ls`
- **THEN** the listing is of `~/Projects/imp`, not the launch directory

#### Scenario: Changed root
- **WHEN** the user runs `/cd /tmp` and the model then runs `pwd`
- **THEN** the command prints `/tmp`

#### Scenario: Relative root
- **WHEN** the user runs `imp -C ..` from `~/Projects/imp`
- **THEN** the session root is `~/Projects`

#### Scenario: Missing directory
- **WHEN** the user runs `imp --cwd /nope`
- **THEN** imp exits with an error naming the path and no session starts

#### Scenario: Typed shell command
- **WHEN** the user types `!pwd` at the prompt
- **THEN** it prints the same root the model's `bash` calls use

### Requirement: Bash calls can carry a label

The system SHALL accept an optional `description` on `bash`, SHALL show it as the tool row summary in place of the command, SHALL still show the command in expanded and high-verbosity rows, and SHALL name the command in any confirmation for a labelled call.

#### Scenario: Labelled call
- **WHEN** the model runs a long pipeline with the description `count photos by month`
- **THEN** the row reads `count photos by month` and Ctrl+O shows the command

#### Scenario: Confirmation
- **WHEN** a labelled call needs confirmation
- **THEN** the prompt names the command, not only the verb

### Requirement: Live sessions share one buffer budget

The system SHALL apply the 16 MB memory and 64 MB spill limits for output buffers across all live sessions together, SHALL keep each session's addresses separate, and SHALL drop a closed session's buffers.

#### Scenario: Shared memory
- **WHEN** two live sessions hold 10 MB of buffers each
- **THEN** the least recently used buffers move to spill until memory holds at most 16 MB

#### Scenario: Separate addresses
- **WHEN** one session reads `imp://out/1` from another session
- **THEN** the result reports the address as unknown

#### Scenario: Closed session
- **WHEN** the user closes a session in the inbox
- **THEN** its buffers are freed

### Requirement: Background jobs end with their session

The system SHALL stop every running background job of a session when that session closes to make room or imp exits, by signalling the job's process group.

#### Scenario: Closed for room
- **WHEN** a session with a running background job closes to make room for a fifth
- **THEN** the job's process group receives SIGTERM

#### Scenario: Exit
- **WHEN** the user quits imp while a session has a running background job
- **THEN** the job's process group receives SIGTERM before imp exits

#### Scenario: Finished jobs
- **WHEN** a session whose jobs have all exited closes
- **THEN** nothing is signalled

### Requirement: Fetch calls APIs

The system SHALL accept a `method` of GET, POST, PUT, PATCH, or DELETE, `headers` as names and values, and a `body` as text or JSON, SHALL send an object body as JSON with its content type, SHALL add `Location`, `Retry-After`, and rate-limit response headers to the result when present, and SHALL return an error's body with its status.

#### Scenario: JSON body
- **WHEN** the model fetches with `method` POST and `body` `{"title":"x"}`
- **THEN** the request carries that JSON with `Content-Type: application/json`

#### Scenario: Error body
- **WHEN** the API answers 422 with a JSON error
- **THEN** the result is a failure holding the status and the error body

#### Scenario: Created
- **WHEN** the API answers 201 with a `Location` header
- **THEN** the result names the location

### Requirement: Credentials come from the config by host

The system SHALL read `auth = <host> <Header>: <value>` lines from the config, SHALL expand `$NAME` and `${NAME}` in the value from the environment when sending, SHALL add the header only when the request's host and port match exactly, SHALL NOT follow a redirect from a request that carries one, SHALL treat each named variable as a secret, and SHALL refuse with the variable's name when it is unset.

#### Scenario: Right host
- **WHEN** the config holds `auth = api.github.com Authorization: Bearer $GITHUB_TOKEN` and the model fetches `https://api.github.com/user`
- **THEN** the request carries the header and the model never sees the token

#### Scenario: Other host
- **WHEN** the model fetches `https://evil.example/?u=api.github.com`
- **THEN** no credential is sent

#### Scenario: Redirect
- **WHEN** a credentialed request is answered with a redirect
- **THEN** the redirect is returned and not followed

#### Scenario: Unset
- **WHEN** `GITHUB_TOKEN` is not set
- **THEN** the call is refused and names `GITHUB_TOKEN`

### Requirement: Agents can check back on jobs

The system SHALL let the model schedule a check-in with `job check` and `after_s` between 30 seconds and 24 hours, optionally for one job, SHALL let `bash` set `notify` to be told when that job ends, SHALL wake the session at that time with a turn that opens with a note from imp, not the user, giving the job's state and latest output, SHALL keep at most four pending check-ins per session, SHALL drop them when the session ends, and SHALL refuse them in line mode.

#### Scenario: Check-in
- **WHEN** the model starts a copy as job 3 and runs `job check` with `after_s` 600
- **THEN** ten minutes later a turn starts with a note naming job 3, its elapsed time, and its last output line

#### Scenario: Told when done
- **WHEN** the model starts a job with `notify` and the job exits while the session is idle
- **THEN** a turn starts with a note giving the exit code and the output's tail

#### Scenario: Limits
- **WHEN** four check-ins are pending, or `after_s` is under 30
- **THEN** the check is refused with the limit named

#### Scenario: Line mode
- **WHEN** the model runs `job check` in line mode
- **THEN** it is refused and told check-ins need the chat

### Requirement: The agent can show output to the user

The system SHALL accept `show` on `bash`, SHALL show the command's output to the user, up to 200 lines, with its colour kept, SHALL give the model the output as plain text, and SHALL give the model only a placeholder naming the size when more than a third of the visible characters are block or braille characters.

#### Scenario: A chart
- **WHEN** the model runs a command that prints a coloured chart with `show`
- **THEN** the user sees the chart in colour and the model receives its text

#### Scenario: A picture
- **WHEN** the model runs `chafa --format symbols --size 60x24 photo.jpg` with `show`
- **THEN** the user sees the picture and the model receives `[shown to the user: a 60×24 picture]`

### Requirement: Agent commands see the screen size

The system SHALL set `COLUMNS` and `LINES` to the current terminal size in the environment of commands the agent runs, when that size is known, and SHALL leave commands the user types unchanged.

#### Scenario: Picture sized to the screen
- **WHEN** the agent runs `echo "$COLUMNS $LINES"` in an 80 by 24 terminal
- **THEN** the output is `80 24`

#### Scenario: No terminal
- **WHEN** imp runs in line mode with no terminal size
- **THEN** the command sees the variables it inherited

### Requirement: Each process has a private scratch folder

The system SHALL create a scratch folder per process under its state directory, SHALL pass its path to agent commands as `IMP_TMP`, SHALL name it in the prompt, SHALL let read, write, and edit reach it, SHALL remove it when the process exits, and SHALL remove at start the folders of processes that no longer run.

#### Scenario: Agent writes scratch
- **WHEN** the agent writes `$IMP_TMP/notes.txt` in normal mode
- **THEN** the write succeeds without a prompt

#### Scenario: Exit cleans up
- **WHEN** imp exits
- **THEN** its scratch folder no longer exists

#### Scenario: Live folders stay
- **WHEN** imp starts while another imp process runs
- **THEN** the other process's scratch folder is kept

### Requirement: Jail refusals do not ask

The system SHALL refuse a write or edit whose path the jail refuses without asking the user, SHALL return the jail's reason, and SHALL treat the scratch folder as inside the session directory for the write prompt.

#### Scenario: Outside path
- **WHEN** the model edits a file in another project in normal mode
- **THEN** no prompt appears and the result says the path is outside the session directory

#### Scenario: Scratch write
- **WHEN** the model writes `$IMP_TMP/notes.txt` in normal mode
- **THEN** no prompt appears

### Requirement: Cat counts as a read

The system SHALL treat a file printed whole by a safe `cat`, with at most `-n` and plain paths, as read once the command exits 0, so a later write may overwrite it.

#### Scenario: Cat then write
- **WHEN** the model runs `cat -n README.md` and then writes README.md
- **THEN** the write is not refused for an unread file

### Requirement: A command can run in the foreground

The system SHALL, for a `bash` call with `foreground: true` in the chat, ask before running it in `normal` and `strict` regardless of allows, SHALL then hand the terminal to the command with no timeout, SHALL restore the interface when it exits, SHALL return only its exit code, and SHALL refuse the call in line mode.

#### Scenario: Run an app
- **WHEN** the model calls `bash` with `htop` and `foreground: true`, and the user answers yes
- **THEN** htop has the screen and keyboard, and when it exits the chat redraws and the result reads `exit 0 (ran in the foreground; output not captured)`

#### Scenario: Allowed shape still asks
- **WHEN** `npm run dev` is allowed always and the model runs it with `foreground: true` in `normal`
- **THEN** imp asks before running it

#### Scenario: Yolo
- **WHEN** the mode is `yolo`
- **THEN** the command runs in the foreground without asking

#### Scenario: Refused
- **WHEN** the user answers no
- **THEN** nothing runs and the result says it was refused

#### Scenario: Line mode
- **WHEN** a foreground call arrives in line mode
- **THEN** it is refused with `foreground needs the chat interface`
