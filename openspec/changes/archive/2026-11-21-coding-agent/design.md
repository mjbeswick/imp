# Design

- Quiet redirects are blanked, byte for byte, in both the masked and the
  original command before the safe check, so offsets still line up. Only
  whole tokens count: `2>/dev/null`, `>/dev/null`, `1>/dev/null`,
  `&>/dev/null`, `2>&1`, and the same with a space before `/dev/null`.
- `cd` targets come from the quote-stripped words after each `cd`. A
  relative target without `..` stays inside. An absolute, `~`, or `..` target
  is resolved lexically and compared with the root. `$`, `-`, or a bare `cd`
  counts as leaving. Only an Allow is raised to Ask; denials stand.
- `shape` of `a | f1 | f2` is the shape of `a` when every later part passes
  the safe-pipeline check. Strict and Deny are unchanged.
- cat reads: each chain part whose words are `cat`, optional `-n`, and plain
  paths with no glob or `$`. Paths go through `resolve`, so the jail still
  applies, and are marked read only when the command exits 0.
