# Design: external skills

## Comparable implementation

The bundled skills in `assets/skills/<name>/SKILL.md` (`rg`, `jq`, `gh`, and others) are
the pattern: one folder per skill, `name` and `description` frontmatter, a short body.
Those are compiled in through `builtin_skills` in `src/store.rs`. This change does not
follow that route for the new skills, for the reasons below.

## Decisions

**Location: `optional-skills/`, not `assets/skills/`.** Files under `assets/` are
compiled in, and the skills here need external programs, so they are opt-in. Judgement
call: a separate folder keeps core and optional skills apart. Cost if wrong: a later move
to `assets/` changes paths only.

**Installation: a documented copy.** A user copies a folder to `~/.config/imp/skills/`.
No new command, no change to `imp init`, which per its spec writes exactly one example.
Judgement call: the smallest change that meets the goal. Cost if wrong: a user may miss
the copy step; the README names it.

**No new code path.** The loader already reads `SKILL.md` folders from the config
directory, so the skills need no Rust change. The one code-adjacent item is a test that
parses each shipped skill with the loader's frontmatter rules and checks the body is
under 4 KB, so a skill cannot silently break.

**Program detection in the body.** Each skill tells the model to run `command -v <tool>`
before using the tool, and to report the install hint if it is missing. Detection stays
in the model's `bash` calls, which keeps the binary free of tool-specific logic.

**browse.** `fetch` is the default. The skill uses obscura only for pages that return an
empty shell without JavaScript. The skill does not enable stealth or bot-protection modes,
and it tells the model not to use them against sites that forbid automated access.

**show-image and show-chart.** Both set `show: true` on `bash`. The renderer
(`chafa` for images; a chart tool or plain text for charts) is detected first. Expected
binary change: zero bytes, because nothing is compiled in. `scripts/size.sh` should report
no change.

## Validation

- Unit test: each `optional-skills/*/SKILL.md` parses with name and description, and its
  body is under 4096 bytes.
- `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- Manual check: copy one skill into a temp config directory and confirm the loader lists
  it (run in a scratch directory outside the repo).
