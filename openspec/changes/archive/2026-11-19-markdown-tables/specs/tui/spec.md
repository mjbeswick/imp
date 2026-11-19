# Spec Delta

## MODIFIED Requirements

### Requirement: Assistant text renders as markdown

The system SHALL render assistant text as a markdown subset: headings, bold, italic, inline code, fenced code, bullet and numbered lists, blockquotes, horizontal rules, and tables, and SHALL NOT render images.

#### Scenario: Emphasis
- **WHEN** the model writes `**done** in 2 s`
- **THEN** the row shows `done` in bold and the rest unchanged

#### Scenario: Fenced code
- **WHEN** the model writes a fenced block
- **THEN** its lines are shown dim, without fence markers, and without wrapping inside the block

#### Scenario: Lists and quotes
- **WHEN** the model writes a bullet list and a quote
- **THEN** bullets render with a marker and the quote renders dim and prefixed

#### Scenario: Table
- **WHEN** the model writes a markdown table
- **THEN** it is drawn as a boxed table rather than raw pipes

## ADDED Requirements

### Requirement: Tables are drawn to fit

The system SHALL draw a markdown table with box lines in the border colour and bold header cells, SHALL align columns as the separator row says, SHALL render inline markdown inside cells, SHALL shrink the widest columns and wrap their cells when the table is wider than the screen, and SHALL show each row as `header: value` lines when the columns cannot fit.

#### Scenario: Fits
- **WHEN** a three-column table fits the screen
- **THEN** it has a top line, a line under the header, and a bottom line, and every row is one line

#### Scenario: Too wide
- **WHEN** a table's natural width exceeds the screen
- **THEN** its widest column wraps inside its cell and no row is wider than the screen

#### Scenario: Far too wide
- **WHEN** even three-character columns cannot fit
- **THEN** each row becomes lines of `header: value`

#### Scenario: Piped
- **WHEN** line mode prints a reply with a table
- **THEN** the table stays as markdown text

### Requirement: Shown text cannot drive the terminal

The system SHALL remove control characters and escape sequences, other than newlines and tabs, from model and tool text before showing it in either front-end, SHALL show tabs as spaces in the chat, and SHALL keep its own escape sequences for colour, links, and layout.

#### Scenario: Clipboard escape
- **WHEN** a reply contains an OSC 52 clipboard sequence
- **THEN** nothing reaches the terminal but the surrounding text

#### Scenario: Colour codes in tool output
- **WHEN** a command prints ANSI colour codes
- **THEN** its preview shows the plain text without the codes
