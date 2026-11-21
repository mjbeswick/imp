## ADDED Requirements

### Requirement: Tool rows name paths from the session directory

The system SHALL show a path under the session directory in a tool row relative to that directory, and a path under the scratch folder as `$IMP_TMP`, both live and when a session is replayed.

#### Scenario: A read inside the project

- **WHEN** the agent reads `/home/m/shop/src/cart.py` in a session rooted at `/home/m/shop`
- **THEN** the tool row reads `read src/cart.py`

#### Scenario: A download into scratch

- **WHEN** a labelled call writes into the scratch folder
- **THEN** the tool row names it under `$IMP_TMP`
