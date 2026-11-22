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

The system SHALL pair each run of removed lines with the run of added lines that follows it, line by line, SHALL draw the words that differ within a pair with a stronger tint and bold, and SHALL leave unpaired lines with the plain tint.

#### Scenario: One word
- **WHEN** an edit changes `let count = 3;` to `let count = 4;`
- **THEN** only `3` and `4` carry the stronger tint and bold

#### Scenario: Unpaired
- **WHEN** an edit adds two lines and removes none
- **THEN** both added lines have the plain tint with no word emphasis

### Requirement: Diff code is syntax-styled

The system SHALL style diff rows by the file's extension for Rust, shell, Python, JavaScript, TypeScript, TOML, and YAML, drawing keywords bold, strings in the success colour, numbers in the accent colour, and comments dim and italic, SHALL leave other extensions unstyled, and SHALL keep changed-word emphasis over these styles.

#### Scenario: Rust
- **WHEN** an edit to `main.rs` adds `let x = "hi"; // note`
- **THEN** `let` is bold, `"hi"` is in the success colour, and `// note` is dim and italic

#### Scenario: Unknown extension
- **WHEN** an edit to `notes.txt` adds a line
- **THEN** it has the tint and no syntax styling

#### Scenario: Multi-line string
- **WHEN** a Python docstring spans three added lines
- **THEN** only its first line is styled as a string
