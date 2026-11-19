# tools Specification

## Purpose
Defines the five built-in tools — shell, read, edit, write, search — their batching, their output caps, the path jail, and the confirmation gate. Owns what the model can do and what it costs in prompt space.

## Requirements

### Requirement: The tool set is small and shell-first

The system SHALL expose exactly `bash`, `read`, `edit`, `write`, `search`, `job`, and `ask` as its tool set, SHALL NOT expose per-CLI tools such as `gh`, `docker`, `kubectl`, `git`, glob, grep, list, patch, or todo, and SHALL teach specific command-line tools through skills instead.

#### Scenario: Default prompt
- **WHEN** a session starts with memory and skills configured
- **THEN** the request carries the seven tools plus `memory` and `skill_load`, and no per-CLI tool

#### Scenario: A CLI is not installed as a tool
- **WHEN** the model needs `gh`
- **THEN** it loads the `gh` skill if one is installed and runs `gh` through `bash`, rather than calling a `gh` tool

#### Scenario: File discovery without a glob tool
- **WHEN** the model needs to find files by pattern
- **THEN** it uses `bash` and the task completes without a dedicated tool

### Requirement: Asking a yes/no question is one keystroke

The system SHALL render a question whose only choices are yes and no as a `y`/`n` prompt, and SHALL return the chosen word.

#### Scenario: Confirmation-style question
- **WHEN** the model asks a question with options `["Yes", "No"]`
- **THEN** pressing `y` answers Yes and pressing `n` answers No

#### Scenario: Escape
- **WHEN** the user presses Esc on any question
- **THEN** the result says the question was not answered

### Requirement: The agent can ask the user a question

The system SHALL provide an `ask` tool taking a question and optional two-to-six choices, SHALL block until the user answers, and SHALL return either the chosen option or the user's typed answer.

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
- **THEN** the tool result says the user did not answer and the turn continues

#### Scenario: Too many options
- **WHEN** the model supplies more than six choices
- **THEN** the call is rejected with a message asking for fewer

### Requirement: Shell commands run non-interactively

The system SHALL run `bash` commands through `sh -c` with stdin on the null device, a 30 second timeout, and a process group, and SHALL return the exit code.

#### Scenario: Prompting command
- **WHEN** a command would wait for terminal input
- **THEN** it reads end of file immediately instead of hanging the agent

#### Scenario: Timeout
- **WHEN** a command runs longer than 30 seconds
- **THEN** the process group is terminated and the result reports a timeout

#### Scenario: Cancel
- **WHEN** the user cancels while a command runs
- **THEN** the process group gets SIGTERM, then SIGKILL after two seconds, and the result reads cancelled

### Requirement: Long commands run as background jobs

The system SHALL start a `bash` command as a job when `background` is set, SHALL return a job id and its output path immediately, SHALL cap `job wait` at 55 seconds and return partial output, and SHALL report a finished job to the model as a `system` note at the tail.

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

### Requirement: Large results spill to disk

The system SHALL return at most 4 KB of any tool result inline, SHALL write larger output to a file under the session's output directory, and SHALL include that path, the original byte count, and a hint to read or search it.

#### Scenario: Big command output
- **WHEN** a command writes 1 MB to stdout
- **THEN** the result holds a short head and tail, the spill path, and the original size

#### Scenario: Spill file readable
- **WHEN** the model reads the spill path from a previous result
- **THEN** the content is available through `read` or `bash`

### Requirement: Reads are batched and numbered

The system SHALL accept one or more paths in a single `read` call, return each file with line numbers, default to 2,000 lines per file, and state clearly when a path is missing, a directory, or not valid UTF-8.

#### Scenario: Three files at once
- **WHEN** the model reads three paths in one call
- **THEN** all three appear in one result and no second call is needed

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

The system SHALL classify every tool action before executing it, SHALL refuse actions the policy denies, SHALL wait for the user on actions the policy asks about, and SHALL return the refusal to the model as a result rather than an error.

#### Scenario: Denied by policy
- **WHEN** a command matches the deny list
- **THEN** it is not run and the model is told it was refused

#### Scenario: Asked and approved
- **WHEN** an action requires confirmation and the user approves
- **THEN** it runs and the result is returned normally

#### Scenario: Asked and refused
- **WHEN** the user refuses
- **THEN** nothing happens and the model receives a result saying the user declined

### Requirement: Search returns summaries, not pages

The system SHALL provide a `search` tool with a `search` operation that queries the web and returns at most five results containing title, URL, summary, date, and a short snippet, and SHALL NOT request or return page bodies in that operation.

#### Scenario: Web question
- **WHEN** the model searches for a current fact
- **THEN** it receives summaries with URLs in one round trip and no page content

#### Scenario: No results
- **WHEN** a search returns an empty result list
- **THEN** the result says so plainly and is reported as unbilled

### Requirement: Fetch returns page text as markdown

The system SHALL provide a `fetch` operation that returns a single page as markdown, starting with the page title and then the page text, SHALL subject the result to the same 4 KB inline cap and spill rule as other tools, and SHALL NOT prefix it with a summary or a heading outline.

#### Scenario: Reading an article
- **WHEN** the model fetches a URL
- **THEN** it receives the page title and markdown text, or an error naming why the page could not be read

#### Scenario: A markdown file
- **WHEN** the model fetches a `.md` file with headings
- **THEN** the result begins with its title or first line of text, not with a list of its headings

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
