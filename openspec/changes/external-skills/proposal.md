# Proposal: external skills

## Why

imp keeps a short tool list on purpose. Browsing JavaScript-rendered pages, showing
images, and drawing Mermaid diagrams are useful, but each needs a third-party program.
Built into the binary they would grow it and tie it to those programs. Shipped as
optional skills, they cost nothing until a user installs the tool and copies the skill.

## What Changes

- Three optional skills ship in the repository under `optional-skills/`, one folder
  each: `browse`, `image`, and `mermaid`.
- Each skill teaches `bash` to use an external program. It checks the program is on
  `PATH` before use and says how to install it if not. imp installs nothing.
- `browse` uses obscura for pages that need JavaScript. `fetch` stays the default for
  everything else.
- `image` uses chafa to draw an image in the terminal. `mermaid` uses graphs-tui to
  draw a Mermaid diagram as Unicode or ASCII. Both use the existing `show` option on
  `bash`.
- Each skill body stays under the 4 KB load cap, so loading it never truncates.
- No credential values appear in any skill. A skill that needs a key names the
  environment variable it reads.
- Installation is a documented copy into `~/.config/imp/skills/`. `imp init` does not
  change.

## Out of Scope

- Compiling these skills into the binary, or adding them to `builtin_skills`.
- Installing, updating, or version-pinning any external program.
- Changes to `fetch`, `bash`, `show`, or the skill loader.
- Stealth or bot-protection features of any browser tool.
- Other chart types (bar, line) beyond what graphs-tui draws. Data plots need their own tool and a later change.
