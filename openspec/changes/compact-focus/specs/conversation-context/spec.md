## ADDED Requirements

### Requirement: A compaction request can carry focus instructions

The system SHALL add the text after a user's `/compact` to the summary request as focus instructions, SHALL run the summary with no focus when no text follows, SHALL trim the text and cut it at 500 characters, SHALL keep the summary's length limit and the last four exchanges unchanged, and SHALL report a cut focus.

#### Scenario: Focus added
- **WHEN** the user runs `/compact keep the auth decisions`
- **THEN** the summary request names `keep the auth decisions` as focus, and the summary still covers the whole older range

#### Scenario: No focus
- **WHEN** the user runs `/compact` with no text
- **THEN** the summary request is the same as before this change

#### Scenario: Long focus
- **WHEN** the focus text is longer than 500 characters
- **THEN** the first 500 characters are sent and the reply says `focus cut to 500 characters`

#### Scenario: Recent exchanges kept
- **WHEN** a focused compaction runs
- **THEN** the last four exchanges appear unchanged after the summary, as without focus

#### Scenario: Automatic compaction
- **WHEN** the token budget triggers compaction
- **THEN** no focus is sent
