## MODIFIED Requirements

### Requirement: The status line reports tokens and cost

The system SHALL show the model name, the served model when it differs, the estimated context tokens, and the cost of this process's messages, SHALL show the share of the context window in use as a whole percent after the context tokens when the window size is known, SHALL cap that percent at 100, SHALL refresh the figures after every request, SHALL abbreviate 1,000 as K and 1,000,000 as M with one decimal, SHALL render cost with three decimals below $0.10 and two above, SHALL NOT show the cache hit share, SHALL show the permission mode only when it is not `normal`, and SHALL omit a figure it cannot compute.

#### Scenario: Abbreviated tokens
- **WHEN** the estimated context is 3,400 tokens
- **THEN** the status line shows `3.4K tok`, and a context of 1,200,000 shows `1.2M tok`

#### Scenario: Small counts stay exact
- **WHEN** the estimated context is 940 tokens
- **THEN** the status line shows `940 tok`

#### Scenario: Context percent
- **WHEN** the estimated context is 12,300 tokens and the context window is 200,000 tokens
- **THEN** the status line shows `12.3K tok (6%)`

#### Scenario: Percent rounds down
- **WHEN** the estimated context is 11,999 tokens and the context window is 200,000 tokens
- **THEN** the status line shows `12K tok (5%)`

#### Scenario: Percent capped
- **WHEN** the estimated context is 250,000 tokens and the context window is 200,000 tokens
- **THEN** the status line shows `250K tok (100%)`

#### Scenario: Window from config
- **WHEN** the config sets `context_window = 32000` for the configured model and the estimated context is 8,000 tokens
- **THEN** the status line shows `8K tok (25%)`, whatever the table holds for that model

#### Scenario: Window unknown
- **WHEN** the configured model is not in the table and the config sets no `context_window`
- **THEN** the status line shows `7.7K tok` with no percent

#### Scenario: Served model differs
- **WHEN** the configured model is `deepseek-flash` and a response reports `deepseek-flash-2026-11`
- **THEN** the status line shows `deepseek-flash (deepseek-flash-2026-11)`

#### Scenario: Served model matches
- **WHEN** the response reports the same string as the configured model, or no response has arrived
- **THEN** the status line shows the configured name alone

#### Scenario: Small cost
- **WHEN** the messages sent in this process cost 0.0014
- **THEN** the status line shows `$0.001`

#### Scenario: Two-decimal cost
- **WHEN** the messages sent in this process cost 0.4246
- **THEN** the status line shows `$0.42`

#### Scenario: Cost accumulates
- **WHEN** a second response costs 0.0531
- **THEN** the status line shows the sum, `$0.48`, not the last response alone

#### Scenario: Mid-turn tokens
- **WHEN** a request finishes and the context is 7,700 tokens
- **THEN** the status line shows `7.7K tok` before the turn ends, with its percent when the window is known

#### Scenario: Removed signals
- **WHEN** any frame is drawn
- **THEN** the status line contains no cache percentage, and no permission mode while the mode is `normal`

#### Scenario: Unknown model
- **WHEN** the configured model has no price entry
- **THEN** the status line shows token counts and no cost
