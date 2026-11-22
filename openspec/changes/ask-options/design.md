# Design

## Approaches

- **Descriptions, multiple choice, and the recommendation convention
  (chosen).** The parts of opencode's tool that help on a narrow screen.
- **Several questions per call, as opencode allows.** A form of questions
  does not fit 54 columns; the model can ask again. Rejected.
- **Parallel `descriptions` array.** Easy for the model to misalign.
  Rejected for objects.

## Schema

`options` items are `{label: string, description?: string}`. The parser
also accepts a bare string as a label, so older habits keep working.
`multiple` is an optional boolean. Two to six options, as today.

## Drawing

Each option row: `[n] label` then `  description` muted, wrapped at the
option's indent. Under `multiple`, a marked option shows `[x]` in place of
its number until sent; the keys row reads `1-n mark · Enter send · e answer
· Esc stop`.

## Result

A single choice returns the label. Multiple choice returns the marked labels
joined by `, `, in option order. A typed answer returns the text.

## Size

Expected: about 1.5 KB.
