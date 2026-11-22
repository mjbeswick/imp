## MODIFIED Requirements

### Requirement: The interface uses colour with meaning

The system SHALL colour by role rather than by text, SHALL paint inline code in the code colour on a tint of that colour and not bold, SHALL keep fenced blocks and quotes muted, SHALL detect truecolor, 256-colour, and no-colour terminals, SHALL honour `NO_COLOR` and a `color=auto|always|never` setting, and SHALL keep bold, italic, underline, and reverse under the `mono` theme while emitting no colour.

#### Scenario: Truecolor terminal
- **WHEN** `COLORTERM` advertises truecolor
- **THEN** roles render as 24-bit RGB

#### Scenario: Plain terminal
- **WHEN** the terminal supports 256 colours only
- **THEN** the same roles render from a 256-colour palette that keeps the same contrast relationships

#### Scenario: No colour
- **WHEN** `NO_COLOR` is set or the setting is `never`
- **THEN** no escape sequences are emitted and the layout is unchanged

#### Scenario: Error stands out
- **WHEN** a tool exits non-zero or a request fails
- **THEN** that line renders in the error role and remains readable without colour

#### Scenario: Inline code is not dim
- **WHEN** a reply contains `` `bash` `` inside a sentence
- **THEN** that span is in the code colour on its tint, not bold and not the muted grey

#### Scenario: Fenced code stays dim
- **WHEN** a reply contains a fenced block
- **THEN** its lines are dim and not bold

#### Scenario: Mono keeps emphasis
- **WHEN** the theme is `mono` and a reply has bold text and inline code
- **THEN** the bold text is bold, the code is reversed, and no colour sequence is emitted

#### Scenario: Mono picker
- **WHEN** a picker is open under `mono`
- **THEN** the highlighted row is reversed

## ADDED Requirements

### Requirement: Markdown structure is styled

The system SHALL paint `#` headings bold, underlined, and in the accent colour, `##` bold and accent, and `###` bold, SHALL underline links, SHALL lead each fenced code line with `│ ` in the border colour, SHALL paint list markers in the accent colour, SHALL set quote text in italic as well as dim, and SHALL draw a horizontal rule across the full width in the border colour.

#### Scenario: Heading levels
- **WHEN** a reply has `# A`, `## B`, and `### C`
- **THEN** `A` is bold, underlined, and accent, `B` is bold and accent, and `C` is bold only

#### Scenario: Link
- **WHEN** a reply has `[docs](https://example.com)`
- **THEN** `docs` is underlined in the link colour

#### Scenario: Fence gutter
- **WHEN** a reply has a fenced block
- **THEN** every body row starts with `│ ` in the border colour and no row is wider than the column

#### Scenario: Rule
- **WHEN** a reply has `---` at 54 columns
- **THEN** the row is 54 `─` in the border colour

### Requirement: Interface rows carry emphasis

The system SHALL set thinking rows in italic, SHALL paint picker descriptions muted on rows that are not highlighted, SHALL paint the picker title bold, SHALL paint the status line's unseen and waiting counts in the accent colour, and SHALL show `/help` section titles bold with keys and commands in the accent colour.

#### Scenario: Thinking
- **WHEN** a thinking row is shown in a colour terminal
- **THEN** it is italic and muted

#### Scenario: Picker description
- **WHEN** the command picker is open
- **THEN** descriptions on rows that are not highlighted are muted

#### Scenario: Unseen count
- **WHEN** rows arrive below the view
- **THEN** the `↑ n` count is in the accent colour and the rest of the status line stays muted

#### Scenario: Help headings
- **WHEN** the user runs `/help` in a colour terminal
- **THEN** each section title is bold and each key and command is accent
