## ADDED Requirements

### Requirement: Bind slash commands to keys and chords

The system SHALL let the user bind a key or a two-key chord to a slash command with `/bind`, SHALL run the
bound command when its key or chord is pressed, SHALL remove a binding with `/unbind`, SHALL restore the
built-in keys with `/keys reset`, SHALL refuse a key that imp uses itself, and SHALL save bindings in the
config as `bind.<chord>` lines.

#### Scenario: Bind a chord with a command
- **WHEN** the user types `/bind ctrl+x n /model`
- **THEN** `bind.ctrl-x-n = /model` is saved, and a confirmation line names the chord

#### Scenario: Capture and confirm
- **WHEN** the user types `/bind`, presses Ctrl+X then n, and presses Enter
- **THEN** the chord is shown while it is captured, and Enter saves it for the command the user names next

#### Scenario: Capture and cancel
- **WHEN** the user types `/bind`, presses Ctrl+X, and presses Esc
- **THEN** nothing is saved and the composer is unchanged

#### Scenario: Run a bound chord
- **WHEN** the user has bound `ctrl+x n` to `/model` and presses Ctrl+X then n within 1.5 seconds
- **THEN** the model picker opens, as if `/model` had been typed

#### Scenario: Chord timeout
- **WHEN** the user presses Ctrl+X and waits longer than 1.5 seconds
- **THEN** Ctrl+X does what it did before the binding, and the next key is handled normally

#### Scenario: Reserved key
- **WHEN** the user binds Ctrl+C, Esc or Enter
- **THEN** the binding is refused with a message, and no line is saved

#### Scenario: Chord from a key used elsewhere
- **WHEN** the user has bound `ctrl+x n` to `/model`, and presses Ctrl+X with a message queued
- **THEN** Ctrl+X waits up to 1.5 seconds; if n follows, the model picker opens, and otherwise Ctrl+X deletes the queued message as before

#### Scenario: Unbind
- **WHEN** the user types `/unbind ctrl+x n`
- **THEN** the binding is removed from the config, and the chord no longer runs a command

#### Scenario: Reset
- **WHEN** the user has several bindings and types `/keys reset`
- **THEN** every `bind.*` line is removed and the built-in keys are the only keys in use

#### Scenario: Keys card lists bindings
- **WHEN** the user has a binding and opens `/keys`
- **THEN** the card lists the binding after the built-in keys

### Requirement: Command lists show bindings

The system SHALL show each command's bound key or chord beside the command in the slash command list and in
the command palette, SHALL leave the description of a command with no binding unchanged, and SHALL update the
shown key when a binding is added, removed or reset.

#### Scenario: Bound command in the slash list
- **WHEN** the user has bound `ctrl+x n` to `/model` and types `/` in the composer
- **THEN** the `/model` row shows `ctrl+x n` beside its description

#### Scenario: Bound command in the palette
- **WHEN** the user has bound `ctrl+p` to `/skills` and opens the command palette with Ctrl+P
- **THEN** the `/skills` row shows `ctrl+p` beside its description

#### Scenario: Unbound command
- **WHEN** a command has no binding
- **THEN** its row shows its description only

#### Scenario: Reset updates the list
- **WHEN** the user runs `/keys reset` and then opens the slash list
- **THEN** no row shows a user binding
