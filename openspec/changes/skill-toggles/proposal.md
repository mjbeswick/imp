# Proposal: skill toggles

## Why

Every skill found on disk is listed in the prompt and can be loaded. A user who has
installed optional skills, or who wants to keep a skill out of a session, has no way to
turn one off without deleting its folder.

## What Changes

- Each skill has an enabled flag stored in the config as `skill.<name>.enabled = false`.
  Skills are enabled by default, so existing configs keep working unchanged.
- A disabled skill is left out of the prompt index and refused by `skill_load`.
- A `/skills` command opens a picker listing each skill with its source and state. Enter
  or space toggles it, and the change is saved to the config.
- `/reload` applies the saved state to the running session.

## Out of Scope

- Per-project toggles stored in `.imp/` or `.agents/`. The flag lives only in the global config.
- Enabling or disabling tools other than skills.
- Editing skill files from the picker.
