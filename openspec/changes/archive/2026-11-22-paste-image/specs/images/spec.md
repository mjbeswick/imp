## ADDED Requirements

### Requirement: Images paste from the clipboard

The system SHALL, on Ctrl+V in the chat, read the system clipboard for an image through `osascript` on macOS, `wl-paste` under Wayland, or `xclip` under X11, SHALL attach an image it finds with the same checks, limits, folder, and marker as a path attachment, SHALL otherwise paste the last copy as before, SHALL skip the clipboard over SSH, SHALL give up on the clipboard after two seconds, and SHALL say once per run when no clipboard tool is available.

#### Scenario: Screenshot on the clipboard
- **WHEN** a PNG is on the macOS clipboard and the user presses Ctrl+V
- **THEN** the transcript shows `[image pasted.png · …]` and the image goes with the next message

#### Scenario: Text on the clipboard
- **WHEN** the clipboard holds text and imp has a last copy
- **THEN** Ctrl+V inserts the last copy into the draft and attaches nothing

#### Scenario: Over SSH
- **WHEN** imp runs with `SSH_CONNECTION` set
- **THEN** Ctrl+V runs no clipboard command

#### Scenario: No tool
- **WHEN** neither `wl-paste` nor `xclip` exists and the user presses Ctrl+V twice
- **THEN** one line says images cannot be pasted here, and it does not repeat

### Requirement: The last attachment can be removed

The system SHALL remove the most recent unsent attachment when the user presses Backspace on an empty draft, and SHALL say which one was removed.

#### Scenario: Remove
- **WHEN** an image is attached and the draft is empty and the user presses Backspace
- **THEN** the transcript says `removed [image pasted.png · …]` and the next message carries no image
