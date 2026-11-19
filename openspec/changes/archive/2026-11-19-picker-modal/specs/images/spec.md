# Spec Delta

## MODIFIED Requirements

### Requirement: Images attach from a path

The system SHALL attach an image when the user runs `/image <path>` or submits a composer token that names an existing image file, SHALL open a browser rooted at the session directory listing directories and image files when `/image` is given no path in the chat interface, SHALL accept PNG, JPEG, GIF, and WebP, SHALL refuse a missing file, an unsupported type, a file over 10 MB, or a fifth image in one message, and SHALL leave the composer text and the pending attachments unchanged on refusal. When removing a pasted path leaves no text, the system SHALL send a short caption instead.

#### Scenario: Explicit command
- **WHEN** the user runs `/image ~/Pictures/shot.png`
- **THEN** the file is attached to the next message and a marker is shown

#### Scenario: Bare command
- **WHEN** the user runs `/image` with no path in the chat interface
- **THEN** a browser opens at the session root listing directories and image files

#### Scenario: Browse and attach
- **WHEN** the user highlights `shot.png` in the image browser and presses Enter
- **THEN** the browser closes and the image is attached to the next message

#### Scenario: Line mode bare command
- **WHEN** the user runs `/image` with no path in line mode
- **THEN** the usage `image <path>` prints and nothing is attached

#### Scenario: Pasted path
- **WHEN** the user pastes `/Users/michael/Pictures/shot.png` and presses Enter with no other text
- **THEN** the image is attached and the model receives a caption with it

#### Scenario: Path plus question
- **WHEN** the user submits `what is wrong here? /tmp/shot.png`
- **THEN** the model receives the question as text and the image as an attachment

#### Scenario: Missing file
- **WHEN** the user runs `/image /tmp/nope.png`
- **THEN** the transcript reports that the file does not exist and nothing is attached

#### Scenario: Too large
- **WHEN** the file is 12 MB
- **THEN** the transcript refuses it and names the 10 MB limit

#### Scenario: Too many
- **WHEN** a fifth image is offered for one message
- **THEN** the transcript refuses it and the first four stay attached
