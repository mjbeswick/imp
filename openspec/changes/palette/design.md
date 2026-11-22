# Design

## Approaches

- **One palette merging commands and actions (chosen).** One list with the
  shortcut beside each row; Enter does the thing.
- **A second list just for keys.** Two lists drift; the keys card already
  exists for reference.
- **Ctrl+P only when one session exists.** The key would mean two things
  depending on pane count, which is worse than losing the back step.

## Details

`palette_items` maps every `COMMANDS` row to `cmd:<name>` and adds the
runnable actions the keys card describes: next session, stop, expand tools,
thinking, clear view, page, keys card, release the mouse, quit. Each action
carries its shortcut in `desc`.

The picker uses a new `PickKind::Palette`; Enter reads the value prefix and
either calls `run_command` or replays the key through `handle_key_inner`.
Filtering is the picker's own.

Session cycling keeps `Ctrl+N` forward. With at most four live sessions,
forward alone reaches any of them, and `Ctrl+O` jumps to a waiting one.

## Size

Expected: under 3 KB.
