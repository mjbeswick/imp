# Design

## Approaches

1. **Move the whole session root.** One value drives the shell, file tools, jail, instructions, skills, memory, and prompt. Chosen: `--cwd` already defines exactly that set, so `/cd` reuses it instead of inventing a second notion of directory.
2. **Move only the shell directory.** Cheap, but file tools and instructions would drift from the shell, and `/cwd` would be ambiguous.
3. **Start a new session on every change.** Loses the conversation the user is working in.

For persistence:

1. **Sidecar plus resume precedence.** The sidecar already has a `cwd` field that nothing reads. Chosen: write it on start and on `/cd`, read it on resume.
2. **Log event only.** Works, but the list and picker still show the old directory.
3. **No persistence.** A resumed session lands wherever it was relaunched, which the request rules out.

## Decisions

- One resolver, `resolve_root(raw, base)`, expands a leading `~` or `~/`, joins a relative path to the base, canonicalises, and rejects a missing path or a file. `--cwd` passes the launch directory as the base; `/cd` passes the current root.
- Root precedence on resume is explicit `--cwd`, then the sidecar's `cwd`, then the launch directory. A recorded directory that no longer exists falls back to the launch directory and prints a note.
- `Session::set_root(path)` sets `tools.root`, calls the existing `reload()` so skills, memory, instructions, and the system prompt come from the new directory, writes the sidecar's `cwd`, and logs a `cwd` event. History is untouched.
- The path jail is `tools.root`, so it moves with the root. This matches launching with `--cwd <path>` and needs no extra rule.
- In the full-screen interface, `/cd` resolves and validates locally, sends `Cmd::SetRoot`, and waits for `Ev::Root(path)` before changing the App's shell root. The worker reloads, writes the sidecar, and reports failure without changing anything on screen.
- `!` commands build a fresh `Tools` from the App root, so they follow the same change.
- Line mode calls `Session::set_root` directly and prints the result.
- `/cd` with no argument shows `cd <path>`. Everything after `/cd ` is the path, spaces included.
- The command table grows to twelve. Help, README, and the help-command count test change with it.
- Tests use temp directories and never touch the real home directory: resolver (`~`, relative, missing, file), sidecar update, resume precedence, TUI `/cwd` and `/cd`, line mode output.
