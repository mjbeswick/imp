## ADDED Requirements

### Requirement: Fetch calls APIs

The system SHALL accept a `method` of GET, POST, PUT, PATCH, or DELETE, `headers` as names and values, and a `body` as text or JSON, SHALL send an object body as JSON with its content type, SHALL add `Location`, `Retry-After`, and rate-limit response headers to the result when present, and SHALL return an error's body with its status.

#### Scenario: JSON body
- **WHEN** the model fetches with `method` POST and `body` `{"title":"x"}`
- **THEN** the request carries that JSON with `Content-Type: application/json`

#### Scenario: Error body
- **WHEN** the API answers 422 with a JSON error
- **THEN** the result is a failure holding the status and the error body

#### Scenario: Created
- **WHEN** the API answers 201 with a `Location` header
- **THEN** the result names the location

### Requirement: Credentials come from the config by host

The system SHALL read `auth = <host> <Header>: <value>` lines from the config, SHALL expand `$NAME` and `${NAME}` in the value from the environment when sending, SHALL add the header only when the request's host and port match exactly, SHALL NOT follow a redirect from a request that carries one, SHALL treat each named variable as a secret, and SHALL refuse with the variable's name when it is unset.

#### Scenario: Right host
- **WHEN** the config holds `auth = api.github.com Authorization: Bearer $GITHUB_TOKEN` and the model fetches `https://api.github.com/user`
- **THEN** the request carries the header and the model never sees the token

#### Scenario: Other host
- **WHEN** the model fetches `https://evil.example/?u=api.github.com`
- **THEN** no credential is sent

#### Scenario: Redirect
- **WHEN** a credentialed request is answered with a redirect
- **THEN** the redirect is returned and not followed

#### Scenario: Unset
- **WHEN** `GITHUB_TOKEN` is not set
- **THEN** the call is refused and names `GITHUB_TOKEN`
