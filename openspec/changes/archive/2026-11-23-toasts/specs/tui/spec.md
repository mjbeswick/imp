## ADDED Requirements

### Requirement: Confirmations are toasts

The system SHALL show confirmations of what the user just did, namely how much was copied and the new thinking setting, theme, verbosity, or model, in the status line for three seconds instead of the transcript, SHALL then restore the status line with one repaint, SHALL keep errors and notes the user must act on in the transcript, and SHALL print confirmations as lines in line mode.

#### Scenario: Copy
- **WHEN** the user copies a 42-character reply
- **THEN** the status line reads `copied 42 characters` and the transcript gains no row

#### Scenario: Expiry
- **WHEN** three seconds pass with no key
- **THEN** the status line shows its usual content again

## MODIFIED Requirements

### Requirement: Text can be copied with /copy

The system SHALL copy the last assistant reply for `/copy` and the last fenced code block for `/copy code`, SHALL report what was copied, and SHALL report when there is nothing to copy.

#### Scenario: Last reply
- **WHEN** the user runs `/copy` after a 120-character reply
- **THEN** that reply is copied and a toast reads `copied 120 characters`

#### Scenario: Last code block
- **WHEN** the user runs `/copy code`
- **THEN** the code of the newest fenced block is copied

#### Scenario: Nothing yet
- **WHEN** the user runs `/copy` before any reply
- **THEN** an info row reads `nothing to copy` and nothing is sent to the model
