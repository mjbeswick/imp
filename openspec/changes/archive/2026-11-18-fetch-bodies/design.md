# Design

## Approaches

1. **Return the body only.** The outline was added to help navigation and turned
   into noise: it eats the first 120 characters the model reads and the first
   row the user sees. The 4 KB cap and the spill path already cover size.
   Chosen.
2. **Keep the outline but move it after the body.** Still tokens the model has
   to skip, and still the first thing in a 4 KB head.
3. **An `outline` flag on the op.** A knob nobody will set, and the model would
   have to know to set it.

## Decisions

- `fetch` returns `title` plus the capped body. No heading scan, no `outline:`
  prefix.
- `outline_of` stays only if nothing else uses it. If it becomes dead, delete it
  rather than leave an unused function.
- The prompt gains one line under Tools: fetch a URL with `search`; `curl` only
  for a plain-text endpoint, an authenticated call, or a stream; never scrape a
  feed with `curl | grep | sed`.
- No retry, cap, or spill change.
