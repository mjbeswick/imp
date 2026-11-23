## ADDED Requirements

### Requirement: The provider dialect is chosen from a list

The system SHALL offer the dialect field of the provider form as one of `deepseek`, `openai`, `anthropic`, `gemini`, `ollama`, and `custom`, SHALL step through them with Left, Right, or Space, SHALL ignore typed text in that field, and SHALL NOT save a dialect that is not in the list.

#### Scenario: Step the dialect
- **WHEN** the dialect field reads `openai` and the user presses Right
- **THEN** the field reads `anthropic`

#### Scenario: Typed text ignored
- **WHEN** the user types `opnai` into the dialect field
- **THEN** the field is unchanged

### Requirement: Tab on the model field opens a picker of known models

The system SHALL open a picker on Tab in the model field of the provider form, listing the default model for the chosen dialect, the recently used models, and a row `type another`, SHALL fill the field with the chosen model and return to the form, and SHALL keep the field's free text when `type another` is chosen.

#### Scenario: Choose a recent model
- **WHEN** the dialect is `deepseek`, the model field is focused, and the user presses Tab and chooses `deepseek-flash`
- **THEN** the field reads `deepseek-flash` and the form is open

#### Scenario: Type another
- **WHEN** the user presses Tab and chooses `type another`
- **THEN** the form returns with the model field focused and its text unchanged

### Requirement: The model field still accepts typed names

The system SHALL keep the model field editable as free text without opening the picker, so a model not in the list can be entered.

#### Scenario: Typed model
- **WHEN** the user types `my-model-7` into the model field
- **THEN** the field reads `my-model-7` and no picker opens
