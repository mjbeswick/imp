# images Specification

## Purpose

Lets a user attach image files to a message from a terminal composer and carries those images with the session.

## Requirements

### Requirement: Images attach from a path

The system SHALL attach an image when the user runs `/image <path>` or submits a composer token naming an image, SHALL open a browser at the session root when `/image` has no path in chat, SHALL accept PNG, JPEG, GIF, and WebP, SHALL refuse a missing file, an unsupported type, a file over 10 MB, or a fifth image, and SHALL leave the composer unchanged on refusal. When a pasted path leaves no text, it SHALL send a short caption.

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

### Requirement: Images are copied into the state directory

The system SHALL copy each accepted image into `~/.local/state/imp/images/`, SHALL name the copy from its content hash and extension, SHALL record the copy's file name, media type, and byte size with the message, SHALL reuse an existing copy with the same hash, and SHALL reattach the recorded copies when a session is resumed.

#### Scenario: Copy
- **WHEN** an image is attached
- **THEN** a file exists under the state images directory and the transcript record names it

#### Scenario: Duplicate
- **WHEN** the same image is attached twice
- **THEN** one copy exists and both messages reference it

#### Scenario: Resume
- **WHEN** a session with an image is resumed
- **THEN** the next request can still send that image

### Requirement: Images travel as content parts

The system SHALL send a user message with attachments as an array of content parts, text first and one `image_url` part per image carrying a base64 data URL, and SHALL keep sending a plain string when a message has no attachments.

#### Scenario: Wire shape
- **WHEN** a message has text and one PNG
- **THEN** its JSON content is an array with a text part and an `image_url` part whose URL starts `data:image/png;base64,`

#### Scenario: Text only
- **WHEN** a message has no attachments
- **THEN** its JSON content is the same string it is today

### Requirement: A rejected image is dropped and retried once

The system SHALL detect an API rejection that names image content, SHALL remove the attachments from that user message, SHALL retry the same request once without them, and SHALL report in the transcript that the model refused the image and the text continued alone.

#### Scenario: Model refuses images
- **WHEN** the API rejects a message with image parts
- **THEN** one retry is sent without images and the transcript says the attachment was dropped

#### Scenario: Retry fails
- **WHEN** the text-only retry also fails
- **THEN** the second error is reported and no further retry runs

### Requirement: The transcript marks attachments

The system SHALL draw one dim line per attachment containing its file name and human-readable size, and SHALL draw no inline pixels.

#### Scenario: Marker
- **WHEN** `shot.png` of 120 KB is attached
- **THEN** a dim line reads `[image shot.png · 117 KB]`

#### Scenario: No pixels
- **WHEN** the terminal supports inline images
- **THEN** the interface still draws only the marker line
