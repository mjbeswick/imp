# Design

## Approaches

- **An anchor and the caret (chosen).** The selection is the bytes between
  an anchor, set by the first Shift move, and the caret. One `Option<usize>`
  per session, kept with the draft when sessions switch.
- **A separate selection mode, as vi's visual mode.** Needs a key to enter
  it and Esc to leave it, and Esc already stops the turn. Rejected.
- **Mouse selection in the composer.** Ctrl+G already hands the mouse to the
  terminal for selecting text. Rejected for now.

## Keys

The decoder reads `CSI 1;2` with `A B C D H F` as the Shift forms of
Up, Down, Right, Left, Home, and End, and `CSI 3~` or `CSI 3;2~` as Delete.
Other modifiers keep their plain meaning, as today. Home and End also come
as `CSI 1~ 7~` and `CSI 4~ 8~` from tmux, screen, and rxvt, which imp did not
read before; a live run under tmux showed Home doing nothing.

Shift+↑/↓ move between rows of the draft only; they never recall history.

## Editing

Before a key reaches the composer: if it is not a Shift move and a selection
exists, Backspace and Delete remove it and stop there; a character, a
newline, or a Ctrl+V with something to paste remove it and then insert. Any
key that is not a Shift move then clears the anchor.

## Drawing

Each draft row gets start and end marks where the selection crosses it, and
the painter turns them into `ESC[7m` and `ESC[27m`. The caret sits outside
the selection: after it when selecting right, before it when selecting left.
Reverse video, as the caret already uses, so the selection shows in `mono`
and with colour off.

## Size

Expected: under 1.5 KB. Measured: `__text` +2024, `__cstring` +0, measured with the uncommitted diff-style theme helpers in the build.
