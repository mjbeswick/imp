# Design

## Approaches

1. **Mode-aware decisions in `perm` plus two guards in `agent`.** Chosen: the
   prompts come from `Decision::Ask` in `perm` and from two direct
   `hooks.confirm` calls for memory; changing those covers every path.
2. **Auto-approve in the hooks when in `yolo`.** Rejected: it hides the rule
   from `perm`, where every other permission decision lives.
3. **A `yolo` label in the status line.** Rejected by the user; colour only.

## Decisions

- `perm::self_config` and `perm::shell_touches_self_config` return `Allow` in
  `yolo` and keep `Deny` in the other modes.
- `perm::classify_command` returns `Allow` for `sudo` in `yolo` when
  `allow_sudo=true`; deny-list matches and `allow_sudo=false` still refuse.
- The memory tool save and compaction fact offers skip `hooks.confirm` when the
  live mode is `yolo`.
- The chat paints the `> ` glyph and the composer rule in `Role::Error` while
  the live mode is `yolo`. A waiting prompt keeps the accent rule. Line mode
  paints its `> ` the same way. With colour off nothing changes.

## Tests

- `perm` unit tests for own config and `sudo` in each mode.
- An agent test: a memory save in `yolo` calls `confirm` zero times.
- A colour frame test: `yolo` paints the glyph and rule in the error role, and
  `normal` does not.
