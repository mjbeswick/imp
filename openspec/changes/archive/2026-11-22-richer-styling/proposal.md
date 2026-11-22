# Proposal

## Why

Markdown structure reads flat: three heading levels look alike, inline code
and bold differ only by colour, and fences, quotes, and rules barely stand
apart from prose. `mono` drops bold and italic along with colour, so code and
emphasis vanish and the picker loses its highlight. The in-app `/help` is one
accent block with no headings.

## What Changes

- `mono` keeps bold, italic, underline, and reverse; only colour is off.
  `NO_COLOR` and `color=never` still emit no escapes.
- The selected picker row is reversed under `mono`.
- Inline code is text on a tint of the code colour, not bold.
- `#` is bold, underlined, and accent; `##` bold and accent; `###` bold.
- Links are underlined.
- Fenced code lines carry a `│ ` gutter in the border colour.
- List markers are accent; quote text is italic; a rule spans the width in
  the border colour.
- Thinking rows and other muted asides are italic.
- Picker descriptions are muted and its title bold.
- The status line's unseen and waiting counts are accent.
- `/help` section titles are bold and its keys and commands accent.

## Keyboard and resources

No keys. kmscon draws bold and underline but likely not italic, so italic
always rides with a colour or dim cue.

## Capabilities

### Modified Capabilities

- `tui`: colour roles, markdown styling, and interface emphasis.
