# Spec Delta

## MODIFIED Requirements

### Requirement: Fetch returns page text as markdown

The system SHALL provide a `fetch` operation that returns a single page as markdown, starting with the page title and then the page text, SHALL subject the result to the same 4 KB inline cap and spill rule as other tools, and SHALL NOT prefix it with a summary or a heading outline.

#### Scenario: Reading an article
- **WHEN** the model fetches a URL
- **THEN** it receives the page title and markdown text, or an error naming why the page could not be read

#### Scenario: A markdown file
- **WHEN** the model fetches a `.md` file with headings
- **THEN** the result begins with its title or first line of text, not with a list of its headings
