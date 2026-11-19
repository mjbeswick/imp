# Spec Delta

## MODIFIED Requirements

### Requirement: Image picker attaches a file

The system SHALL open a browser for `/image` with no argument, SHALL show directories and PNG, JPEG, GIF, and WebP files under the session root, SHALL mark directories as navigable, SHALL open the highlighted directory on Enter, Tab, or Right, SHALL attach the highlighted image on Enter, and SHALL leave the browser open when a key opens a directory.

#### Scenario: Browse and attach
- **WHEN** the user runs `/image`, opens `Pictures`, highlights `shot.png`, and presses Enter
- **THEN** the picker closes and the image is attached to the next message

#### Scenario: Non-image file
- **WHEN** a `.txt` file is present
- **THEN** it is not listed

#### Scenario: Directory
- **WHEN** the highlight is on a directory and the user presses Enter
- **THEN** the browser lists that directory and nothing is attached

#### Scenario: Open with an arrow
- **WHEN** the highlight is on a directory and the user presses Right or Tab
- **THEN** the browser lists that directory and nothing is attached

## REMOVED Requirements

### Requirement: Directory picker can climb above the session root

**Reason**: Enter now chooses and Tab now opens, the reverse of this requirement.
**Migration**: Replaced by "The directory picker chooses with Enter".

### Requirement: The directory list starts with the parent row

**Reason**: The listing now starts with `./` before `../`, and the row keys changed.
**Migration**: Replaced by "The directory list starts with this folder and its parent".

## ADDED Requirements

### Requirement: The directory picker chooses with Enter

The system SHALL open a directory browser for `/cd` with no argument at the session root, SHALL make the highlighted folder the session root on Enter and close, SHALL open the highlighted folder on Tab or Right, SHALL go to the parent on Left, up to the filesystem root, and SHALL change the root only after the choice succeeds.

#### Scenario: Enter chooses
- **WHEN** the highlight is on `src/` and the user presses Enter
- **THEN** the picker closes and the session root becomes the absolute path of `src/`

#### Scenario: Tab opens
- **WHEN** the highlight is on `src/` and the user presses Tab or Right
- **THEN** the browser lists `src/` and the session root is unchanged

#### Scenario: Parent
- **WHEN** the user presses Left inside `src/`
- **THEN** the browser returns to the directory above it

#### Scenario: Filesystem root
- **WHEN** the browser is at `/` and the user presses Left
- **THEN** nothing moves

#### Scenario: Filter and choose
- **WHEN** the user types `pro` and presses Enter
- **THEN** the first matching folder becomes the session root

#### Scenario: Typed path
- **WHEN** the user runs `/cd ~/Projects/imp`
- **THEN** the root changes directly and no browser opens

### Requirement: The directory list starts with this folder and its parent

The system SHALL begin every `/cd` listing with a `./` row for the folder being browsed, followed by a `../` row when a parent exists, SHALL make either the session root on Enter, SHALL open the parent when Tab or Right is pressed on `../`, SHALL ignore Tab and Right on `./`, and SHALL show the keys as `Enter choose · Tab/→ open · ← up · Esc close`.

#### Scenario: Row order
- **WHEN** the user opens `/cd` in a folder with subfolders
- **THEN** the rows read `./`, `../`, then the subfolders

#### Scenario: Leaf folder
- **WHEN** the user opens a folder with no subfolders and presses Enter on `./`
- **THEN** that folder becomes the session root

#### Scenario: Filesystem root
- **WHEN** the browser lists `/`
- **THEN** `./` is the only fixed row and no `../` appears

#### Scenario: Image browser
- **WHEN** the user opens `/image`
- **THEN** neither `./` nor `../` appears
