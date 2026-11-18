# skills Specification

## Purpose
Loads reusable instructions from disk, keeps their cost near zero until they are needed, and lets a project override a global skill of the same name.

## Requirements

### Requirement: Skills are discovered from directories

The system SHALL discover each skill as a directory containing `SKILL.md` with frontmatter naming the skill and describing when to use it, reading a global directory, a project directory, and any additional directory passed at launch.

#### Scenario: Existing skill set reused
- **WHEN** the user points the extra skills directory at an existing collection of `SKILL.md` folders
- **THEN** those skills are discovered without conversion

#### Scenario: Malformed frontmatter
- **WHEN** a `SKILL.md` lacks a name or description
- **THEN** that skill is skipped and the interface reports it once

### Requirement: Only the index is in the prompt

The system SHALL include at most 40 skills in the prompt, each as one line of name and description, and SHALL NOT include body text until the skill is loaded.

#### Scenario: Twenty skills installed
- **WHEN** a session starts with twenty skills
- **THEN** the prompt grows by twenty short lines rather than twenty documents

### Requirement: Loading a skill returns its body

The system SHALL provide a load operation returning the skill body, capped at 4 KB, as a tool result that becomes part of the conversation history.

#### Scenario: Load then use
- **WHEN** the model loads a skill and then runs a command the skill describes
- **THEN** the body is present in history at the time of the call

#### Scenario: Oversized body
- **WHEN** a skill body exceeds 4 KB
- **THEN** the result is truncated with an explicit marker

### Requirement: Project skills override global skills

The system SHALL prefer a project skill over a global skill when both have the same name, and SHALL list the source path for each skill in the interface.

#### Scenario: Shadowed skill
- **WHEN** the same name exists globally and in the project
- **THEN** loading it returns the project version

### Requirement: Empty history left behind

The system SHALL run with no skills directory present, and SHALL treat a skill directory populated after launch as unavailable until reload.

#### Scenario: No skills
- **WHEN** neither directory exists
- **THEN** the session starts normally with no skill section in the prompt
