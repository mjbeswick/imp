## ADDED Requirements

### Requirement: The draft edits by word and by row

The system SHALL move the caret by word on Option+←/→ and Alt+B/F, SHALL delete the word before the caret on Option+Backspace and Ctrl+W and the word after it on Option+Delete and Alt+D, SHALL extend the selection by word on Shift+Option+←/→, SHALL go to the start of the caret's row on Ctrl+A, SHALL delete to the start of the row on Ctrl+U and to its end on Ctrl+K, and SHALL list these keys on the keys card.

#### Scenario: Delete a word
- **WHEN** the draft is `run the tests` with the caret at the end and the user presses Option+Backspace
- **THEN** the draft is `run the `

#### Scenario: Replace a word
- **WHEN** the caret is after `npm` and the user presses Shift+Option+← and types `pnpm`
- **THEN** `npm` is replaced by `pnpm`

#### Scenario: Line keys
- **WHEN** the draft's second row is `second` with the caret after `sec` and the user presses Ctrl+K
- **THEN** that row is `sec` and the first row is unchanged

#### Scenario: Esc alone
- **WHEN** the user presses Esc and nothing follows
- **THEN** it acts as Esc, not as Alt
