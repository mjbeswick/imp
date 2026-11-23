## ADDED Requirements

### Requirement: The settings modal steps the enumerable settings it adds

The system SHALL list `allow_sudo`, `color`, `links`, and `title_summary` as rows of the settings modal, SHALL step each through its values with Right, Enter, Space, or Left, SHALL apply each change at once, and SHALL save it to the config file keeping every other line and comment.

#### Scenario: Step a value
- **WHEN** the user highlights `color`, which reads `auto`, and presses Enter
- **THEN** the row reads `always`, the session applies it, and the config file gains `color = always`

#### Scenario: Step back
- **WHEN** the user highlights `links` and presses Left
- **THEN** the row moves to the previous value and the change is saved

### Requirement: A text or number setting opens a one-field form

The system SHALL open a one-field form on Enter for a text or number row of the settings modal, SHALL save and apply a value that the config accepts, SHALL keep the form open with the reason when the value is refused, and SHALL close the form without saving on Esc.

#### Scenario: Valid number
- **WHEN** the user opens the `max_tokens` row, types `4096`, and presses Enter
- **THEN** the row reads `4096`, the session applies it, and the config file holds `max_tokens = 4096`

#### Scenario: Refused number
- **WHEN** the user types `0` for `max_steps` and presses Enter
- **THEN** the form stays open and shows that the value is refused, and nothing is saved

#### Scenario: Cancel
- **WHEN** the user types a new model name and presses Esc
- **THEN** the form closes, the row keeps its old value, and the config file is unchanged

### Requirement: The API key is entered masked and never shown

The system SHALL show the `api_key` row as `set` or `not set`, SHALL open a masked field when the user edits it, SHALL NOT display, echo, or record the key in the transcript, the status line, or the config file, and SHALL store it where provider keys are stored.

#### Scenario: Row state
- **WHEN** a key is stored for the active provider
- **THEN** the `api_key` row reads `set` and no part of the key is drawn

#### Scenario: Entry masked
- **WHEN** the user types a key into the masked field
- **THEN** the field shows only bullets, and the key is written to the key store and not to the config file

### Requirement: Grouped settings stay in the config file

The system SHALL NOT list `auth`, `render`, `pass_env`, or `allow` as rows of the settings modal, and SHALL show a footer line in the modal that names the config file as the place to change them.

#### Scenario: Footer names the file
- **WHEN** the settings modal is open
- **THEN** its footer names the config file for the grouped settings
