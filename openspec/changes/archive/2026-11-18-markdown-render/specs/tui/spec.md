# Spec Delta

## MODIFIED Requirements

### Requirement: Output streams as it arrives

The system SHALL render each token within 100 ms of receiving it, keep the newest line visible while streaming, and show a caret at the end of incomplete text.

#### Scenario: Long answer
- **WHEN** an answer longer than the pane arrives over several seconds
- **THEN** text grows continuously and the pane follows the tail

#### Scenario: Scroll detaches
- **WHEN** the user scrolls up during streaming
- **THEN** following stops, a line count of unseen output is shown, and End resumes following

#### Scenario: Half-written markup
- **WHEN** a stream stops in the middle of a link or a fence
- **THEN** the partial line renders as plain text and no broken escape sequence reaches the terminal

## ADDED Requirements

### Requirement: Assistant text renders as markdown

The system SHALL render assistant text as a markdown subset: headings, bold, italic, inline code, fenced code, bullet and numbered lists, blockquotes, and horizontal rules, and SHALL NOT render tables or images.

#### Scenario: Emphasis
- **WHEN** the model writes `**done** in 2 s`
- **THEN** the row shows `done` in bold and the rest unchanged

#### Scenario: Fenced code
- **WHEN** the model writes a fenced block
- **THEN** its lines are shown dim, without fence markers, and without wrapping inside the block

#### Scenario: Lists and quotes
- **WHEN** the model writes a bullet list and a quote
- **THEN** bullets render with a marker and the quote renders dim and prefixed

### Requirement: Links are clickable where the terminal supports it

The system SHALL draw `[text](url)` as the link text wrapped in an OSC 8 hyperlink when the terminal supports them, SHALL keep the URL on the row when it does not, SHALL link bare `https://` URLs as well, and SHALL never split the sequence across wrapped rows.

#### Scenario: Link terminal
- **WHEN** the terminal advertises hyperlink support and the model writes `[docs](https://example.com/x)`
- **THEN** the row shows `docs` inside an OSC 8 sequence for that URL

#### Scenario: Plain terminal
- **WHEN** the terminal does not advertise support
- **THEN** the row shows `docs (https://example.com/x)`

#### Scenario: Wrapped link
- **WHEN** a link's text wraps onto a second row
- **THEN** each row carries its own complete OSC 8 sequence

#### Scenario: Off
- **WHEN** `links=never`
- **THEN** no OSC 8 sequence is emitted and links render as text with the URL
