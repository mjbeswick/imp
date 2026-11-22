# Design

## Approaches

- **Hand-written token rules (chosen).** A few hundred lines for six
  languages, no dependency, cheap per line.
- **A syntax-highlighting crate.** Accurate, but adds hundreds of KB to a
  binary kept as small as possible. Rejected.
- **Word emphasis only.** Cheapest, but the user asked for syntax styling
  too.

## Rows

Diff rows become spans instead of one painted string: `(text, Style)` where
`Style` holds a role, bold, italic, and an emphasis flag. The existing
`Line::Spans` path draws them. The band is padded to `cols` after the last
span, on the row's tint.

## Changed words

`push_diff` buffers a run of `-` lines and the `+` run after it. Pairs are
matched by position. Each pair is split into words and punctuation and run
through a longest-common-subsequence diff; tokens outside it are emphasised.
ponytail: O(n·m) per pair on tokens, bounded by line length; skip emphasis
on lines over 200 characters.

## Syntax

`src/syntax.rs` maps an extension to a rule set: keywords, line-comment
prefix, string quotes. One left-to-right scan per line yields spans. The
path comes from the tool row's arguments, which hold it for `edit` and
`write`, so `push_diff` knows the extension.

Keywords:

| Language | Extensions | Comment | Keywords |
|---|---|---|---|
| Rust | `rs` | `//` | `as break const continue crate else enum fn for if impl in let loop match mod move mut pub ref return self Self static struct trait type unsafe use where while` |
| Shell | `sh bash zsh` | `#` | `if then else elif fi for while do done case esac in function return local export` |
| Python | `py` | `#` | `and as class def elif else except False for from if import in is lambda None not or pass raise return True try while with yield` |
| JS/TS | `js mjs ts tsx jsx` | `//` | `async await break case class const continue default else export extends for from function if import let new return switch this throw try typeof var while` |
| TOML | `toml` | `#` | `true false` |
| YAML | `yml yaml` | `#` | `true false null` |

## Contrast

A syntax colour is used only where it reaches 4.5:1 on its row's tint, in
the active theme and colour depth; otherwise that token takes the text
colour and keeps its bold or italic. The check runs on the RGB values, or on
the 256-colour entries when that is the depth. A measurement before building
found every light theme and most comments below 4.5:1, while the text colour
passes everywhere.

- **Fall back to the text colour (chosen).** No new colours to tune, and
  light themes keep the weight and slant cues.
- **Darker syntax colours for light themes.** Four themes times three slots
  tuned by hand. Rejected.
- **No syntax colour on light themes.** Throws away the dark themes' check
  too. Rejected.

The changed-word tint doubles the plain tint's strength: 44% of the colour
on dark themes, 24% on light. The theme's text colour fell under 4.5:1 on it
in solarized-light even at a third more strength, so changed words are
white on dark themes and black on light, at 6.65:1 or better. In 256 colours
a lifted cube colour can be bright even on a dark theme, so the text there
is whichever of white and black reads better. A test checks every theme at
both depths.

Diff rows carry one mark per character, the syntax kind in the low bits and
a changed flag on top, and wrap by character so the marks stay aligned; a
continuation starts under the code.

## Size

Expected: 3 to 5 KB.
