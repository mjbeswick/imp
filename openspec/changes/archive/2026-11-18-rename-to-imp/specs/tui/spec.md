# Spec Delta

## Purpose

Defines the terminal interface: what the user sees while a turn runs, which keys exist, what the status line reports, and how the terminal is left after any exit.

## MODIFIED Requirements

### Requirement: Two front-ends share one loop

The system SHALL offer a line mode and a full-screen chat mode, SHALL pick the full-screen mode when both stdin and stdout are terminals, SHALL pick the line mode otherwise, and SHALL allow either to be forced with `--cli` or `--chat`.

#### Scenario: Interactive launch
- **WHEN** the user runs `imp` in a terminal
- **THEN** the full-screen chat opens

#### Scenario: Scripted launch
- **WHEN** stdout is a pipe
- **THEN** the line mode runs, printing plain text with no escape sequences

#### Scenario: Forced
- **WHEN** the user passes `--cli` in a terminal
- **THEN** the line mode runs without taking over the screen

#### Scenario: Same behaviour
- **WHEN** the same prompt is sent in either mode
- **THEN** the same tools run and the same history is recorded

### Requirement: Help is grouped, aligned, and shared

The system SHALL render `--help` with titled sections (usage, sessions, running, other, examples, environment, files), aligned columns, defaults in brackets, and descriptions wrapped to the terminal width, and SHALL render `/help` from the same command table in both front-ends.

#### Scenario: Flag help
- **WHEN** the user runs `imp --help` in an 80-column terminal
- **THEN** no line exceeds 80 columns, every flag appears once, and defaults are shown

#### Scenario: Same commands everywhere
- **WHEN** the user runs `/help` in chat or line mode
- **THEN** the same seven commands and the same key list appear

#### Scenario: Unknown command
- **WHEN** the user types `/compac`
- **THEN** the reply suggests `/compact`
