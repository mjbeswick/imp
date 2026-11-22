# Design

## Approaches

- **Net change against the first version (chosen).** Five edits to one file
  read as one change, and nothing outside imp's tools is claimed.
- **Replay every edit's diff.** Long, and repeats lines that later changed
  again. Rejected.
- **`git diff` only.** Needs git and mixes in the user's own changes. Kept
  as `/diff git`.

## Tracking

The session keeps `originals: Vec<(PathBuf, Before)>`, recorded once the
write is allowed and before it happens: `Text` of the file before imp's
first `edit` or `write` touched it, `Missing` when it did not exist, or
`Untracked` when it was over 1 MB or not UTF-8, which is listed, not diffed.

## Diff

`linediff::hunks`: the common start and end are trimmed, and the middle is
diffed by longest common subsequence, grouped into hunks with three lines of
context and `⋯` between them. ponytail: O(n·m) on the middle; past four
million cells it is shown as all removed then all added. A Myers diff if
that bites.

## Display

The rows go through the diff painter from `diff-style`. `/diff git` runs
`git diff --no-color <args>` with the session root as its directory, through
the same timeout as other local commands, and shows its output as diff rows.

## Syntax

Each file's rows use `diff-style`'s syntax rules for its extension. For
`/diff`, the path is known. For `/diff git`, each `+++ b/<path>` header sets
the extension for the rows below it, until the next file. This change needs
`diff-style` built first.

## Order

The command list is modified after `keys-and-cycling`; archive after it.

## Size

Expected: 1.5 to 2.5 KB.
