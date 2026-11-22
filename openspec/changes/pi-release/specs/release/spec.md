## ADDED Requirements

### Requirement: Figures are measured on the target

The project SHALL provide a script that reports binary size, cold start, and idle memory for a local build or an SSH host, and the README SHALL show those figures for the M1 and for the Pi Zero 2 W.

#### Scenario: Measure a host
- **WHEN** the developer runs the measure script with `hackberry`
- **THEN** it prints the size, the median cold start of 30 runs, and the idle RSS from that device

### Requirement: Deploys build off the device

The deploy script SHALL build the target's binary on the developer's machine and copy it to each host, keeping the previous binary as `imp.prev`, and SHALL build on the device only when asked with `--native`.

#### Scenario: Cross deploy
- **WHEN** the developer deploys to `hackberry`
- **THEN** no compiler runs on the device and the new binary answers `--version`

#### Scenario: Native fallback
- **WHEN** the developer deploys with `--native`
- **THEN** the source is copied and built on the device as before
