# Design

## Decisions

- **Shape, not string.** The key is the program plus the first argument that
  does not start with `-`, or the program alone when there is none. An exact
  string would rarely match again; the program alone would let `a` on
  `rm -rf build` approve every `rm`.
- **Simple commands only.** A pipe, `;`, `&&`, `||`, or a redirect can hide a
  second command behind an approved first word, so those prompts offer only
  `y` and `n`.
- **Held by the session.** A `HashSet<String>` beside `fetch_hosts`, cleared
  with it. `Hooks::confirm` returns a three-way answer; existing callers that
  cannot offer `a` treat it as `y`.
- **Line mode** accepts `a` the same way and prints the same hint.

## Keyboard

One letter per answer, no chords. `a` sits beside the home row on the
Hackberry's keyboard.

## Resources

One small set per session. Expected cost under 1.5 KB of binary.
