# External skills

## ADDED Requirements

### Requirement: Optional skills ship outside the binary

The system SHALL ship the `browse`, `image`, and `mermaid` skills under `optional-skills/<name>/SKILL.md`, SHALL NOT compile them into the binary, and SHALL NOT write them during `imp init`.

#### Scenario: Fresh install
- **WHEN** `imp init` runs on a fresh machine
- **THEN** none of the three optional skills is created

#### Scenario: Binary unchanged
- **WHEN** the optional skills are added to the repository
- **THEN** `scripts/size.sh` reports no change in the binary size

### Requirement: Each optional skill is valid and fits the load cap

Each shipped optional skill SHALL have frontmatter with `name` and `description`, SHALL have a body under 4096 bytes, and SHALL be accepted by the existing skill loader.

#### Scenario: Shipped skills parse
- **WHEN** the test suite reads each `optional-skills/*/SKILL.md`
- **THEN** each parses with a name and description and a body under 4096 bytes

#### Scenario: Oversized body
- **WHEN** a skill body would exceed 4096 bytes
- **THEN** the test fails before the change is committed

### Requirement: Skills check for their program before use

Each optional skill SHALL instruct the model to confirm its external program is on `PATH` before using it, and SHALL give the install hint when it is not, and SHALL NOT install anything itself.

#### Scenario: Program missing
- **WHEN** a skill runs and its program is not on `PATH`
- **THEN** the model reports the install hint and does not attempt installation

#### Scenario: Program present
- **WHEN** the program is on `PATH`
- **THEN** the skill proceeds with the documented command

### Requirement: Browsing prefers fetch

The `browse` skill SHALL direct the model to use `fetch` first, SHALL use obscura only when a page needs JavaScript, and SHALL NOT enable stealth or bot-protection modes.

#### Scenario: Static page
- **WHEN** a page is plain HTML
- **THEN** the model uses `fetch` and not obscura

#### Scenario: JavaScript page
- **WHEN** `fetch` returns an empty shell for a page
- **THEN** the model uses obscura to render it

### Requirement: Image and Mermaid skills use show

The `image` skill SHALL render images with chafa, and the `mermaid` skill SHALL render Mermaid diagrams with graphs-tui. Both SHALL display their output through the `show` option of `bash`.

#### Scenario: Image shown
- **WHEN** the user asks to show an image in the terminal
- **THEN** the skill renders it with chafa and `show` set to true

#### Scenario: Mermaid shown
- **WHEN** the user asks to show a Mermaid diagram in the terminal
- **THEN** the skill renders it with graphs-tui and `show` set to true

### Requirement: No credentials in skills

No optional skill SHALL contain a credential value. A skill that needs a key SHALL name the environment variable it reads.

#### Scenario: Keyed service
- **WHEN** a skill needs an API key
- **THEN** it names the variable and contains no value

#### Scenario: Secret check
- **WHEN** the change is committed
- **THEN** a grep of `optional-skills/` finds no token-like string
