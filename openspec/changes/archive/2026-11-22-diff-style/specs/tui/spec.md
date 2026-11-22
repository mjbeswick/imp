## ADDED Requirements

### Requirement: Diff bands reach the screen edge

The system SHALL, with colour on, tint each added and removed diff row from its sign to the last column, leaving the indent before the sign on the terminal background.

#### Scenario: Full band
- **WHEN** a 10-character added line is drawn at 54 columns
- **THEN** the tint covers every column from the `+` to column 54

#### Scenario: No colour
- **WHEN** colour is off
- **THEN** the row is not padded and keeps its `+` or `-`

### Requirement: Changed words stand out

The system SHALL pair each run of removed lines with the run of added lines that follows it, line by line, SHALL draw the words that differ within a pair with a tint twice as strong and bold, SHALL keep text on that stronger tint at a contrast of at least 4.5:1, and SHALL leave unpaired lines with the plain tint.

#### Scenario: One word
- **WHEN** an edit changes `let count = 3;` to `let count = 4;`
- **THEN** only `3` and `4` carry the stronger tint and bold

#### Scenario: Unpaired
- **WHEN** an edit adds two lines and removes none
- **THEN** both added lines have the plain tint with no word emphasis

### Requirement: Diff code is syntax-styled

The system SHALL style diff rows by the file's extension for Rust, shell, Python, JavaScript, TypeScript, TOML, and YAML, drawing keywords bold, strings in the success colour, numbers in the accent colour, and comments in the muted colour and italic, SHALL leave other extensions unstyled, and SHALL keep changed-word emphasis over these styles.

#### Scenario: Rust
- **WHEN** an edit to `main.rs` adds `let x = "hi"; // note`
- **THEN** `let` is bold, `"hi"` is in the success colour, and `// note` is italic, in the muted colour where that reaches 4.5:1 on the tint

#### Scenario: Unknown extension
- **WHEN** an edit to `notes.txt` adds a line
- **THEN** it has the tint and no syntax styling

#### Scenario: Multi-line string
- **WHEN** a Python docstring spans three added lines
- **THEN** only its first line is styled as a string

### Requirement: Syntax colours keep their contrast

The system SHALL use a syntax colour on a diff row only where it reaches a contrast of 4.5:1 on that row's tint in the active theme and colour depth, and SHALL otherwise draw the token in the text colour with its bold or italic kept.

#### Scenario: Faint colour falls back
- **WHEN** the theme is `solarized-light` and an added Rust line holds a string
- **THEN** the string is in the text colour, because the success colour is under 4.5:1 on the added tint

#### Scenario: Strong colour kept
- **WHEN** the theme is `default` and an added Rust line holds a string
- **THEN** the string is in the success colour

## MODIFIED Requirements

### Requirement: File changes show a diff

After a successful `edit` or `write`, the system SHALL draw the changed lines under the tool row, SHALL mark removed lines in the delete colour and added lines in the add colour, SHALL show at most 12 lines by default with a count of the rest, SHALL show the complete diff when the row is expanded with Ctrl+E, and SHALL keep the diff out of the tool result sent to the model.

#### Scenario: Edit diff
- **WHEN** an edit replaces two lines with three
- **THEN** the row shows the two removed lines in red and the three added lines in green

#### Scenario: New file
- **WHEN** `write` creates a file with 20 lines
- **THEN** the preview shows the first 12 as additions and says 8 more follow

#### Scenario: Overwrite
- **WHEN** `write` replaces an existing file
- **THEN** the diff shows the written content as added lines

#### Scenario: Expand
- **WHEN** the user presses Ctrl+E after an edit
- **THEN** the full diff appears, not only the 12-line preview

#### Scenario: No colour
- **WHEN** colour is off
- **THEN** removed lines begin with `-` and added lines with `+`, and the layout is unchanged

#### Scenario: Model result unchanged
- **WHEN** an edit succeeds
- **THEN** the tool result the model receives is still the short `edited <path>` sentence

#### Scenario: Failure
- **WHEN** an edit is refused because a match failed
- **THEN** no diff is drawn and the existing error preview is shown
