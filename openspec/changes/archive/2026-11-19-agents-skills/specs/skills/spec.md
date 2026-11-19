# Spec Delta

## MODIFIED Requirements

### Requirement: Skills are discovered from directories

The system SHALL discover each skill as a directory containing `SKILL.md` with frontmatter naming the skill and describing when to use it, SHALL read the global config directory, the project `.imp/skills`, the project `.agents/skills`, and any additional directory passed at launch, SHALL skip entries that are not directories, and SHALL apply later directories over earlier ones when names collide.

#### Scenario: Existing skill set reused
- **WHEN** the user points the extra skills directory at an existing collection of `SKILL.md` folders
- **THEN** those skills are discovered without conversion

#### Scenario: OpenSpec skills
- **WHEN** a project has `.agents/skills/openspec-propose/SKILL.md`
- **THEN** `openspec-propose` appears in the skill index with its description

#### Scenario: A stray file
- **WHEN** `.agents/skills/.openspec-target` is a file
- **THEN** it is ignored and no skill is reported for it

#### Scenario: Malformed frontmatter
- **WHEN** a `SKILL.md` lacks a name or description
- **THEN** that skill is skipped and the interface reports it once

### Requirement: Project skills override global skills

The system SHALL apply skill sources from least to most specific — global config, `.imp/skills`, `.agents/skills`, then an explicit extra directory — SHALL prefer the later source when two skills share a name, and SHALL list the source path for each skill in the interface.

#### Scenario: Shadowed skill
- **WHEN** the same name exists globally and in the project
- **THEN** loading it returns the project version

#### Scenario: Project sources
- **WHEN** the same name exists in `.imp/skills` and `.agents/skills`
- **THEN** the `.agents` version wins

#### Scenario: Explicit directory
- **WHEN** `--skills-dir` holds a skill with the same name as a project skill
- **THEN** the explicit directory wins
