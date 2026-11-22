## MODIFIED Requirements

### Requirement: The composer grows with its text

The system SHALL draw the composer across one row per newline in the input, SHALL wrap a line wider than the screen at word boundaries onto further rows, SHALL prefix the first row with `> ` and every other row with two spaces, SHALL keep the caret visible on its row, SHALL grow to at most eight rows or half the screen, whichever is smaller, and SHALL move Up and Down by drawn row before recalling history.

#### Scenario: Two rows
- **WHEN** the input holds `line one` and `line two` on separate rows
- **THEN** both rows are visible, the second row starts with two spaces, and the status line stays the last row

#### Scenario: Caret on its row
- **WHEN** the caret sits in the middle of the second row
- **THEN** the caret block is drawn there and no other row carries it

#### Scenario: At the cap
- **WHEN** the input holds more rows than the composer may show
- **THEN** the composer shows only the last rows and keeps the caret row visible

#### Scenario: Wide row
- **WHEN** one input line is 120 characters at 54 columns
- **THEN** it is drawn across three rows broken at spaces, each row starting `> ` or two spaces, and no row is wider than 54

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

