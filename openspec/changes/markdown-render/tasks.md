# Tasks

- [ ] 2.1 Parse inline spans: bold, italic, inline code, `[text](url)`, bare URLs. Verify: tests cover each form and a half-written `[a](ht` stays plain.
- [ ] 2.2 Parse block shapes: heading, bullet, numbered, quote, rule, fenced code. Verify: tests cover each and a fence keeps its body unformatted.
- [ ] 2.3 Wrap spans to a width without counting escapes. Verify: a bold phrase and a link each wrap and every row is within the width.
- [ ] 2.4 Emit OSC 8 per span per row when links are on, and `text (url)` when off. Verify: tests for both, including a wrapped link carrying a complete sequence on each row.
- [ ] 2.5 Add the `links` setting with `auto`, `always`, `never`, `IMP_LINKS`, and terminal detection. Verify: config and detection tests, and the init template names it.
- [ ] 2.6 Render assistant rows through the parser in the TUI. Verify: a frame test shows bold, a bullet, and a code block from one reply.
- [ ] 2.7 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
