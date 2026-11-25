## ADDED Requirements

### Requirement: Model favourites

The system SHALL let the user mark model rows in the `/model` picker as favourites with the `f` key while the filter is empty, SHALL store favourites in a file next to the recent-models file, one `provider::model` value per line, SHALL list favourites first in the model picker, each marked with a star, and SHALL keep the highlight on the same row when a favourite is toggled.

#### Scenario: Mark a model as favourite
- **WHEN** the user opens `/model`, highlights a model row, and presses `f` with an empty filter
- **THEN** the row is marked with a star, the favourite is written to the favourites file, and a toast reads `favourite: <model>`

#### Scenario: Unmark a favourite
- **WHEN** the user presses `f` on a row that is already a favourite
- **THEN** the star is removed, the row is removed from the favourites file, and a toast reads `unfavourite: <model>`

#### Scenario: Favourites stay at the top
- **WHEN** the user opens `/model` with favourites saved from an earlier session
- **THEN** the favourite rows appear first, in the order the catalog lists them, and the other rows follow in their usual order

#### Scenario: Favourite survives a restart
- **WHEN** the user marks a model as favourite, quits, and opens `/model` in a new session
- **THEN** the model is still marked and listed first

#### Scenario: Highlight stays on the toggled row
- **WHEN** the user presses `f` on a row that moves to the top of the list
- **THEN** the highlight stays on that same row

#### Scenario: The f key types into a filter
- **WHEN** the filter already holds text and the user presses `f`
- **THEN** the `f` is added to the filter and no favourite changes

#### Scenario: A favourite whose provider is not usable
- **WHEN** a saved favourite names a provider that is not in the model list
- **THEN** the row is not shown, and the favourite stays in the file for when the provider is usable again

#### Scenario: Write failure
- **WHEN** the favourites file cannot be written
- **THEN** an error row is shown and the picker keeps its previous favourite state
