# Spec Delta

## MODIFIED Requirements

### Requirement: Only two files are written

The system SHALL write exactly one agent example and one skill example, SHALL write a commented config template containing no API key, and SHALL NOT write any session, memory note, or agent other than the examples.

#### Scenario: Template contents
- **WHEN** the config template is created
- **THEN** every meaningful key is present and commented, including `links`, and `api_key` is absent

#### Scenario: Examples are real
- **WHEN** the two example files are read
- **THEN** they are valid files of their kind and would load without edits
