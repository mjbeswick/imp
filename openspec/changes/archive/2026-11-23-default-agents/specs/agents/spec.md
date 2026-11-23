## ADDED Requirements

### Requirement: Three agents are built in

The system SHALL provide built-in agents `build`, with all tools, `plan`, with read and bash tools and instructions to investigate and plan without editing, and `review`, with read and bash tools and instructions to report defects only, SHALL start sessions in `build`, SHALL let an agent file with the same name replace a built-in, and SHALL treat the name `default` as `build`.

#### Scenario: Fresh machine
- **WHEN** no agent files exist and the user presses Tab twice from `build`
- **THEN** the agent becomes `plan`, then `review`

#### Scenario: Override
- **WHEN** `~/.config/imp/agents/plan.md` exists
- **THEN** `plan` uses that file and is listed once

#### Scenario: Old name
- **WHEN** the user runs `imp --agent default`
- **THEN** the session starts in `build`
