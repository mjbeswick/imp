## ADDED Requirements

### Requirement: The composer is a filled block with an accent bar

The system SHALL draw the composer as one block with the panel background and a left accent bar in the accent colour that spans every row of the block, SHALL leave one blank row above and one below the input inside the block, and SHALL draw no prompt glyph before the input.

#### Scenario: Block with blank rows
- **WHEN** the session is idle and the input is empty
- **THEN** the block shows the caret at the input's start with a blank row above and below it

#### Scenario: Accent bar spans the block
- **WHEN** the block holds the input, the blank rows, and the footer row
- **THEN** the left accent bar is drawn on every row of the block, not only on the caret row

### Requirement: The composer footer row names the session

The system SHALL draw the footer row inside the block below the blank row, with the agent name capitalised, the permission mode, the model, the provider, and the reasoning effort, and SHALL start the input and the footer row at the same left column.

#### Scenario: Footer contents
- **WHEN** the session is idle and the input is empty
- **THEN** the footer row reads the agent, mode, model, provider, and effort, for example `Build auto · Claude Haiku 5.5 Tesco LiteLLM (Tescode SSO) · medium`

#### Scenario: Aligned columns
- **WHEN** the footer row is drawn under the input
- **THEN** the input's first column and the footer's first column are the same column

#### Scenario: Accent colour
- **WHEN** the footer row is drawn
- **THEN** the agent name is in the primary accent colour, the reasoning effort is in the warning colour, and the provider is muted

#### Scenario: Shell draft
- **WHEN** the input begins with `!`
- **THEN** the footer names the shell and the input keeps its left edge

### Requirement: Running commands show in a row above the composer

The system SHALL draw a muted row above the composer block that counts the running background commands while any runs, and SHALL draw no row when none runs.

#### Scenario: One command
- **WHEN** one background command is running
- **THEN** the row above the composer reads `1 command`

#### Scenario: No command
- **WHEN** no background command is running
- **THEN** no row is drawn above the composer

#### Scenario: Commands finish
- **WHEN** the last running command finishes
- **THEN** the row disappears on the next frame

### Requirement: Ctrl+P opens the command menu

The system SHALL open the command menu when the user presses Ctrl+P with the composer empty, as typing `/` does, and SHALL leave the composer unchanged on any other key.

#### Scenario: Open with Ctrl+P
- **WHEN** the composer is empty and the user presses Ctrl+P
- **THEN** the command menu opens with no filter and the first command highlighted

#### Scenario: Composer holds text
- **WHEN** the composer holds text and the user presses Ctrl+P
- **THEN** the text is kept and no menu opens

## MODIFIED Requirements

### Requirement: The interface is one chat column

The system SHALL render a scrolling transcript, the composer block with its footer row, and one status bar row at the bottom, SHALL draw no rule above the input, SHALL NOT use panes, tabs, or widgets, SHALL draw no border around the picker or the command menu, and SHALL tell the model the terminal width, and the height when known, before a turn whenever either has changed since the previous turn.

#### Scenario: Quiet layout
- **WHEN** the session is idle and the transcript is empty
- **THEN** the screen shows the welcome row, the composer block with its footer row, and one status bar

#### Scenario: Welcome leaves
- **WHEN** the first message is sent
- **THEN** the welcome row is not drawn again

#### Scenario: Turn spacing
- **WHEN** the transcript holds two user entries
- **THEN** one blank row sits directly above the second and none above the first

#### Scenario: Short terminal
- **WHEN** the terminal has 7 rows
- **THEN** no rule is drawn and the transcript keeps that row

#### Scenario: Long transcript
- **WHEN** the transcript exceeds the screen height
- **THEN** it scrolls within the same column and no second pane appears

#### Scenario: Taller composer
- **WHEN** the composer holds three rows
- **THEN** the transcript gives up two rows, the composer block, its footer row, and the status bar keep their places, and the layout is otherwise unchanged

#### Scenario: Resized window
- **WHEN** the window width changes between turns
- **THEN** the next turn carries one line naming the new width, and later turns at that width carry none

#### Scenario: Taller window
- **WHEN** only the window height changes between turns
- **THEN** the next turn carries one line naming the width and the new height

#### Scenario: Piped output
- **WHEN** input is a pipe with no terminal size
- **THEN** no width line is sent

#### Scenario: One overlay
- **WHEN** the picker is open
- **THEN** it has no border and the chat column resumes unchanged when it closes

### Requirement: The status line reports tokens and cost

The system SHALL draw the status bar as one row at the bottom with the session directory at the left, the goal state next when a goal is set, the estimated context tokens with their share of the context window in the form `674.6K (67%)`, the cost of this process's messages, and the hint `ctrl+p commands` at the right, SHALL show no `/ for commands` hint in the status bar, SHALL show the model name, the served model when it differs, the provider, the permission mode, and the reasoning effort in the footer row under the input, SHALL refresh both figures after every request, SHALL abbreviate 1,000 as K and 1,000,000 as M with one decimal, SHALL render cost with three decimals below $0.10 and two above, SHALL NOT show the cache hit share, SHALL show the permission mode even when it is `normal`, and SHALL omit a figure it cannot compute.

#### Scenario: Status bar layout
- **WHEN** a goal is paused, the context is 674,600 tokens in a 1,000,000-token window, and the cost is $3.04
- **THEN** the status bar reads the directory at the left, then `goal paused`, then `674.6K (67%)`, then `$3.04`, and `ctrl+p commands` at the right

#### Scenario: Abbreviated tokens
- **WHEN** the estimated context is 3,400 tokens in a 128,000-token window
- **THEN** the status bar shows `3.4K (2%)`, the share rounded down, and a context of 1,200,000 in a 1,200,000 window shows `1.2M (100%)`

#### Scenario: Small counts stay exact
- **WHEN** the estimated context is 940 tokens in a 128,000-token window
- **THEN** the status bar shows `940 (0%)`

#### Scenario: Served model differs
- **WHEN** the configured model is `deepseek-flash` and a response reports `deepseek-flash-2026-11`
- **THEN** the footer shows `deepseek-flash (deepseek-flash-2026-11)`

#### Scenario: Served model matches
- **WHEN** the response reports the same string as the configured model, or no response has arrived
- **THEN** the footer shows the configured name alone

#### Scenario: Small cost
- **WHEN** the messages sent in this process cost 0.0014
- **THEN** the status bar shows `$0.001`

#### Scenario: Two-decimal cost
- **WHEN** the messages sent in this process cost 0.4246
- **THEN** the status bar shows `$0.42`

#### Scenario: Cost accumulates
- **WHEN** a second response costs 0.0531
- **THEN** the status bar shows the sum, `$0.48`, not the last response alone

#### Scenario: Mid-turn tokens
- **WHEN** a request finishes and the context is 7,700 tokens
- **THEN** the status bar shows `7.7K (6%)` before the turn ends

#### Scenario: Removed signals
- **WHEN** any frame is drawn
- **THEN** the status bar contains no cache percentage, and the footer shows the permission mode even while it is `normal`

#### Scenario: Unknown model
- **WHEN** the configured model has no price entry
- **THEN** the status bar shows token counts and no cost

### Requirement: Slash commands autocomplete

The system SHALL open the command menu above the composer when the input begins with `/`, SHALL show commands filtered by the word before the first space, SHALL rank prefix matches before substring matches, SHALL show no title row and no footer in the menu, SHALL draw a muted single-column rail at the left and right edge of each menu row, which is not a box border, SHALL draw the menu directly above the composer block with no blank row between them, SHALL draw the highlighted row's name and description in the accent colour on the select background, SHALL accept the highlighted command with Tab or Enter, SHALL open the value picker after a command word and space when that command has values, SHALL seed its filter with the text after the space, and SHALL keep it closed for free-text arguments.

#### Scenario: Rails and gap
- **WHEN** the command menu is open above the composer
- **THEN** each menu row has a muted rail at its left and right edge, and the menu's last row touches the composer block with no blank row between

#### Scenario: Narrowing
- **WHEN** the user types `/co`
- **THEN** `/compact` is listed and other commands are filtered out

#### Scenario: Accept
- **WHEN** the user presses Tab
- **THEN** the highlighted command is completed in the input line and the menu closes

#### Scenario: Run immediately
- **WHEN** the user presses Enter on the highlighted command
- **THEN** the command runs and the input clears

#### Scenario: Dismiss
- **WHEN** the user presses Esc
- **THEN** the menu closes and the typed text stays

#### Scenario: Values follow the command
- **WHEN** the user types `/theme ` and then `gru`
- **THEN** the theme picker opens with `gru` in the filter and `gruvbox-dark` listed

#### Scenario: Free text
- **WHEN** the user types `/stash remember the tests`
- **THEN** no picker opens and the whole text is stashed on Enter

### Requirement: The composer grows with its text

The system SHALL draw the composer across one row per newline in the input, SHALL wrap a line wider than the screen at word boundaries onto further rows, SHALL draw every row at the input's left edge with no prompt glyph, SHALL keep the caret visible on its row, SHALL grow to at most eight rows or half the screen, whichever is smaller, and SHALL move Up and Down by drawn row before recalling history.

#### Scenario: Two rows
- **WHEN** the input holds `line one` and `line two` on separate rows
- **THEN** both rows are visible at the same left edge, and the status bar stays the last row

#### Scenario: Caret on its row
- **WHEN** the caret sits in the middle of the second row
- **THEN** the caret block is drawn there and no other row carries it

#### Scenario: At the cap
- **WHEN** the input holds more rows than the composer may show
- **THEN** the composer shows only the last rows and keeps the caret row visible

#### Scenario: Wide row
- **WHEN** one input line is 120 characters at 54 columns
- **THEN** it is drawn across three rows broken at spaces, each row starting at the left edge, and no row is wider than 54

#### Scenario: Long word
- **WHEN** the input holds one 80-character word at 54 columns
- **THEN** the word breaks mid-word and every row fits

#### Scenario: Up inside a wrapped line
- **WHEN** the caret is on the third drawn row of one wrapped line and the user presses Up
- **THEN** the caret moves to the second drawn row and the draft is unchanged

#### Scenario: Up on the top row
- **WHEN** the caret is on the first drawn row and history holds an entry
- **THEN** Up recalls that entry

#### Scenario: After sending
- **WHEN** a multi-row prompt is sent
- **THEN** the composer returns to one empty row and the transcript shows the prompt

### Requirement: The status line shows the session directory

The system SHALL draw the session directory at the left end of the status bar, SHALL shorten the home directory to `~`, SHALL keep two spaces after it, SHALL shorten a path that does not fit in this order and use the first form that fits: the last two folders whole with earlier parents cut to one character, then the last folder whole with earlier parents as one `…`, then the last folder cut from the front, and SHALL omit it rather than cut the model name or figures.

#### Scenario: Wide terminal
- **WHEN** the terminal is 100 columns and the root is the home directory's `Projects/imp`
- **THEN** the status bar starts with `~/Projects/imp` in the first column

#### Scenario: Narrow terminal
- **WHEN** the root is `~/Projects/imp/src/providers` and 6 columns are left after the figures and two spaces
- **THEN** the directory is absent and the model name is whole

#### Scenario: Abbreviated parents
- **WHEN** the root is `~/Projects/imp/src/providers` and 20 columns are left for it
- **THEN** the status bar starts with `~/P/i/src/providers`, keeping two folders whole

#### Scenario: Last folder only
- **WHEN** the same root has 12 columns left
- **THEN** the status bar starts with `…/providers`

#### Scenario: Dot folder
- **WHEN** the root is `~/.config/imp/agents` and the parents must be cut
- **THEN** `.config` shows as `.c`
