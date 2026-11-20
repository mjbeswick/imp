## MODIFIED Requirements

### Requirement: The inbox lists every live session

The system SHALL open the inbox with Ctrl+O or `/inbox` over the whole transcript area, keeping the status line, and close it with Ctrl+O or Esc, SHALL show one card per live session with its number, full title, state (`needs approval`, `replied`, `working`, or `idle`), and age, and below it what the session needs or is doing, and SHALL order cards by that state list and then newest first.

#### Scenario: Open
- **WHEN** three sessions are live and the user presses Ctrl+O
- **THEN** the inbox shows three cards, the highlight is on the first, and the status line is still drawn

#### Scenario: Order
- **WHEN** one session waits for approval and another has replied
- **THEN** the waiting one is first

#### Scenario: Replied clears
- **WHEN** the user opens a session marked `replied`
- **THEN** it becomes `idle`

#### Scenario: Approval card
- **WHEN** a session waits to run `cargo test --release`
- **THEN** its card shows the command and the answer chips under the title

#### Scenario: Full title
- **WHEN** a session is titled `Give me the weather for Woking` and the screen is 80 columns wide
- **THEN** the card shows the whole title

#### Scenario: Footer follows the highlight
- **WHEN** the highlight is on an idle card
- **THEN** the footer offers open, jump, close, and back, and no answer keys

### Requirement: Act from the inbox

The system SHALL open the highlighted session with Enter and session `n` with the number key `n`, SHALL answer its pending confirmation with `y`, `n`, `s`, or `a` without opening it, following the same rules as the prompt, SHALL close an idle session with `x`, and SHALL show `stop it first` when `x` is pressed on a working or waiting session.

#### Scenario: Approve in place
- **WHEN** a row shows `needs approval` for `run npm test` and the user presses `y`
- **THEN** the command runs in that session, the inbox stays open, and the row becomes `working`

#### Scenario: Approve for the session
- **WHEN** a row shows `needs approval` for `run npm test` and the user presses `s`
- **THEN** the command runs and later `npm test` runs in that session without asking

#### Scenario: Open a session
- **WHEN** the user presses Enter on a row
- **THEN** the inbox closes and that session is in view

#### Scenario: Jump by number
- **WHEN** three sessions are live and the user presses `2`
- **THEN** the inbox closes and session 2 is in view

#### Scenario: Close a session
- **WHEN** the user presses `x` on an idle row
- **THEN** the session leaves the inbox and stays in `/sessions`
