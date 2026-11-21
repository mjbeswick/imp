## MODIFIED Requirements

### Requirement: Fetch returns page text as markdown

The system SHALL provide a local `fetch` tool for every provider that makes one GET request over `http` or `https` with a 15 second limit, SHALL return the status line and the body, SHALL return JSON and text unchanged, SHALL return HTML as markdown with its title, headings, links as absolute URLs, list items, and table rows, without scripts, styles, navigation, footers, asides, or forms, SHALL return RSS and Atom feeds as a list of item titles, dates, and links, and SHALL apply the 4 KB cap and spill rule.

#### Scenario: Reading an article
- **WHEN** the model fetches an HTML page
- **THEN** it receives the status line, the page title as a heading, and the text as markdown without scripts, tags, or navigation

#### Scenario: A markdown file
- **WHEN** the model fetches a `.md` file with headings
- **THEN** the result is the file's text as served, beginning with its first line

#### Scenario: A JSON API
- **WHEN** the model fetches `https://api.open-meteo.com/v1/forecast?...`
- **THEN** it receives `200 application/json` and the JSON body unchanged

#### Scenario: Local network
- **WHEN** the model fetches `http://192.168.0.61:11080/`
- **THEN** the request is made from this machine and the device answers

#### Scenario: Other schemes
- **WHEN** the model fetches `file:///etc/passwd`
- **THEN** it is refused and no request is made

#### Scenario: A feed
- **WHEN** the model fetches `https://feeds.bbci.co.uk/news/rss.xml`
- **THEN** it receives one line per item with its title, date, and link, and no XML

#### Scenario: Links
- **WHEN** a page links `/news/123` with the text `Budget`
- **THEN** the markdown holds `[Budget](https://host/news/123)`
