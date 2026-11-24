# Design: external skills

## Comparable implementation

The bundled skills in `assets/skills/<name>/SKILL.md` (`rg`, `jq`, `gh`, and others) are
the pattern: one folder per skill, `name` and `description` frontmatter, a short body.
Those are compiled in through `builtin_skills` in `src/store.rs`. This change does not
follow that route for the new skills, for the reasons below.

## Decisions

**Location: `optional-skills/`, not `assets/skills/`.** Files under `assets/` are
compiled in, and these skills need external programs, so they are opt-in. Judgement
call: a separate folder keeps core and optional skills apart. Cost if wrong: a later move
to `assets/` changes paths only.

**Installation: the agent copies on request.** The skills stay in `optional-skills/`, their
own folder. When the user asks to install one, the agent copies that folder into
`~/.config/imp/skills/` with `bash`, after checking the destination does not exist. It
never overwrites. No new command and no change to `imp init`, which per its spec writes
exactly one example. Judgement call: the user asked for the agent to do the install, so
the copy uses the existing `bash` tool and needs no Rust change. Cost if wrong: a copy
into a live skills folder is not undone automatically; the no-overwrite check limits this.

**Names: `browse`, `image`, `mermaid`.** The user chose `image` for chafa and `mermaid`
for graphs-tui, replacing the earlier `show-image` and `show-chart`. Judgement call: name
each skill by the content it handles, so the model routes on what the user asked for.

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

**image.** Uses chafa (`chafa --format symbols`) with `show: true` on `bash`. Install hint:
`brew install chafa`. Not installed on this machine, so the skill is untested here.

**mermaid.** Uses graphs-tui, a Rust crate installed with `cargo install graphs-tui`. Output
is Unicode or ASCII text, shown with `show: true`. Not installed here, so it is untested
here. Bar and line charts are out of scope.

**Expected binary change: zero bytes**, because nothing is compiled in. `scripts/size.sh`
should report no change.

## Validation

- Unit test: each `optional-skills/*/SKILL.md` parses with name and description, and its
  body is under 4096 bytes.
- `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- Manual check: copy one skill into a temp config directory and confirm the loader lists
  it (run in a scratch directory outside the repo). Running chafa or graphs-tui needs those
  tools installed, which this machine lacks; that check is reported as not run.
