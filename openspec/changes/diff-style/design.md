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
diff detail carries the path, so `push_diff` knows the extension.

## Size

Expected: 3 to 5 KB.
