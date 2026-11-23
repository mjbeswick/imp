# Design

## Context

- `SETTING_ROWS` in `src/tui.rs` lists the ten rows. `setting_choices` gives the
  values a row steps through, and `step_setting` applies and saves one.
- `Config::set` (`src/config.rs`) already parses every key, so a form only needs
  to pass the typed text through it and report the error it returns.
- `save_setting` replaces one `key =` line and keeps every other line.
- `Pending::ProviderForm` and `Pending::KeyEntry` (masked) already show a
  multi-field form and a masked key row.

## Decision

- Enumerable rows reuse `setting_choices` and `step_setting`. No new code path.
- A text or number row opens `Pending::SettingField { key, value, error }`. Enter
  calls `Config::set`; on `Err` the form stays open with the message in `error`.
  On `Ok` the value is applied, saved with `save_setting`, and sent to the
  session with `Cmd::SetConfig`, as the stepping rows do.
- `api_key` reuses the masked key entry that onboarding uses. Its row shows the
  stored state, never the value, and it saves through the existing key store.
- The modal shows a footer line that names the config file for the grouped
  settings, so a user knows where they live.

## Risks

- A number that parses but is out of range is refused by `Config::set` for
  `max_steps` and `goal_rounds`. The form shows that refusal and stays open.
- A key typed in the masked field can reach the terminal's scrollback if the
  field echoes. The masked field draws bullets only, and the key is not written
  to the transcript.
