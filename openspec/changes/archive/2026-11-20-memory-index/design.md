# Design

## Decisions

- **Titles, not bodies.** A title is usually the fact ("User location: Woking,
  UK"), so the index alone often answers, and a search fetches the rest. Bodies
  in the prompt would grow without bound.
- **Cap of 40.** Same as skills. Forty short lines is about 400 tokens at
  worst, which the Pi Zero's small context budget can afford; notes beyond
  the cap are still found by search.
- **Stable order.** Sorted by slug, project notes before global ones, so the
  prompt prefix stays cacheable between sessions.
- **Title source.** The first `#` heading, else the file stem.
- **Word-wise search.** Split the query on whitespace; a note matches when
  every word appears in its name or body. The excerpt centres on the first
  word. A phrase query still matches as before.

## Resources

One directory read per prompt build, no new state. Expected cost under 1 KB
of binary.
