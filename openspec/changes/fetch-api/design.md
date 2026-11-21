# Design

## Decisions

- **One request function in `http.rs`.** It takes a method, headers, and an
  optional body; `CURLOPT_CUSTOMREQUEST` sets methods other than GET and POST.
  A header callback keeps a short allow-list of response headers.
- **Credentials by host, from the config.** The model never names a variable,
  so injected content cannot send a key to a host of its choosing. Matching is
  on the exact `host[:port]`.
- **No redirects with credentials.** curl forwards custom headers to a
  redirect target, so a credentialed call returns the 3xx instead.
- **Auth variables are secrets** whatever their names, registered at start
  before the secret set is read.
- **Writes ask every time.** The prompt shows only yes or no for them; a
  session-wide yes for writes is left out until it is missed.

## Keyboard

No change.

## Resources

No new process or dependency; one header callback per request.
