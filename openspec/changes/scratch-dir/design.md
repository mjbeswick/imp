# Design

## One folder per process

`<state>/imp/out/tmp/<pid>`. Sessions in one process share it; they belong to
the same user and the same screen. A folder per session would need an id
before the session file exists. The jail already allows `out`, so read, write
and edit need no change.

Cleanup: main removes its folder on exit. At start it removes each
`tmp/<pid>` whose process is gone (`kill(pid, 0)` fails with ESRCH), so a
crash leaves nothing behind for long and a long-running imp in tmux is never
swept by another one.

## Downloads

A download is approved only when it is one command with no pipes, chaining,
redirects, or substitution other than `$IMP_TMP`, the program is `curl` with
the plain-GET flags plus `-o`/`--output`, or `wget` with `-q` and `-O`, and
the target starts with `$IMP_TMP/` and contains no `..`. It is then classified
by `classify_fetch`, so Strict still asks and long URLs still ask.

## Prompt

The baseline is at its byte budget, so the scratch line goes in the composed
prompt, after the date rule.
