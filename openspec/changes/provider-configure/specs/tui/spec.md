## ADDED Requirements

### Requirement: A provider row opens its configuration form

The system SHALL open the provider form when the user presses `e` on a row of the provider picker, and SHALL seed the form from the named profile when one exists, otherwise from the built-in profile's base URL, model, dialect, and key variable.

#### Scenario: Configure a custom provider
- **WHEN** the user highlights a named provider `work` and presses `e`
- **THEN** the form shows its saved base, model, dialect and key variable

#### Scenario: Configure a built-in provider
- **WHEN** the user highlights `deepseek` and presses `e`
- **THEN** the form shows the built-in base URL, model, dialect and key variable

### Requirement: A built-in provider is configured as a named copy

The system SHALL save a provider form for a built-in provider as a named profile, SHALL leave the built-in profile unchanged, and SHALL refuse a name that is a built-in provider name.

#### Scenario: Save a built-in change
- **WHEN** the user highlights `deepseek`, presses `e`, changes the model to `deepseek-pro`, and saves under the name `deepseek-custom`
- **THEN** the config gains that named provider with the new model, the picker still lists the built-in `deepseek`, and its model is unchanged

#### Scenario: Built-in name refused
- **WHEN** the user saves the form with the name `openai`
- **THEN** the form reports that the name is a built-in provider and nothing is saved

### Requirement: Enter opens the form, u switches, and the footer names both

The system SHALL open the configuration form when the user presses Enter on a row of the provider picker, SHALL switch the session to the highlighted provider when the user presses `u` on that row, or Ctrl+U in a provider picker opened by a typed `/provider`, SHALL keep Enter switching during first-run setup, and SHALL show `u use` and `e configure` in the footer of the provider picker when a row is highlighted.

#### Scenario: Enter opens the form
- **WHEN** the user highlights `ollama` in the provider picker and presses Enter
- **THEN** the configuration form for `ollama` opens and the session stays on its current provider

#### Scenario: u switches
- **WHEN** the user highlights `ollama` and presses `u`
- **THEN** the session switches to `ollama` and the picker closes

#### Scenario: Footer
- **WHEN** the provider picker is open with a row highlighted
- **THEN** the footer reads `u use` and `e configure`

### Requirement: Ctrl+E configures a row of a typed provider picker

The system SHALL open the configuration form for the highlighted provider when the user presses Ctrl+E in the provider picker opened by a typed `/provider`, where letters filter the list, SHALL NOT send the key to the composer, and SHALL show `^E configure` in that picker's footer.

#### Scenario: Ctrl+E after typing
- **WHEN** the user types `/provider`, highlights `work`, and presses Ctrl+E
- **THEN** the form for `work` opens and the composer still reads `/provider `

#### Scenario: Footer after typing
- **WHEN** the provider picker was opened by a typed `/provider`
- **THEN** its footer reads `^E configure`, and no `e configure` hint is shown
