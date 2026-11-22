# Proposal

## Why

The status line shows the session directory only when the whole path fits.
Otherwise the directory vanishes. A deep project path disappears even on a
100-column screen, and on the Hackberry most paths are gone.

## What Changes

The directory shortens in steps and stops at the first that fits:

1. The full path, with home as `~`: `~/Projects/imp/src/providers`.
2. Every parent cut to its first character, a dot folder to its dot and
   next character: `~/P/i/s/providers`. The last folder stays whole.
3. The parents as one `…`: `…/providers`.
4. The last folder cut from the front: `…viders`, when at least 8 columns
   are left for it.
5. Otherwise the directory is left out, as today.

The model name and figures still win: the directory never takes their room.

## Keyboard and resources

No keys. One short string built per status line.

## Capabilities

### Modified Capabilities

- `tui`: the status line shortens the directory before leaving it out.
