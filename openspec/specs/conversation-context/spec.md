# conversation-context Specification

## Purpose
Decides what each request contains: a cache-stable prefix, complete assistant records, a bounded conversation tail, and summarization when the tail grows too large.

## Requirements

### Requirement: The prompt prefix is stable across turns

The system SHALL order the request as system prompt, memory index, skills index, then conversation, and SHALL put volatile data such as timestamps, working directory, token counts, and cache statistics nowhere before the conversation history.

#### Scenario: Consecutive turns share a prefix
- **WHEN** the user sends two messages with no memory or skill changes between them
- **THEN** the bytes before the conversation tail are identical in both requests

#### Scenario: Volatile data placement
- **WHEN** the working directory changes between turns
- **THEN** no earlier part of the request changes as a result

### Requirement: Assistant records round-trip intact

The system SHALL store and resend each assistant message's `content`, `reasoning_content`, and `tool_calls`, and SHALL NOT send a tool result whose call identifier is absent from the history.

#### Scenario: Reasoning retained
- **WHEN** a turn produced reasoning content and the next request is built
- **THEN** the stored assistant message still carries that reasoning content

#### Scenario: Orphan tool result
- **WHEN** a tool result exists whose call identifier was removed with an older message
- **THEN** the result is dropped rather than sent

### Requirement: The prefix is frozen for the session

The system SHALL build the prefix once at session start and SHALL NOT rewrite it when memory is saved, a skill is added, or a skill is loaded; such changes SHALL be delivered as a `system` message at the tail of the conversation.

#### Scenario: Note saved mid-session
- **WHEN** the user approves a new memory note during a session
- **THEN** the note arrives as a tail system message and the next request's prefix is unchanged

#### Scenario: Prefix survives
- **WHEN** two requests are built either side of a memory write
- **THEN** the bytes before the conversation tail are identical

### Requirement: Tool output is pruned before summarization

The system SHALL shorten the bodies of tool results older than the last ten messages once the estimate passes 45,000 tokens, replacing each with a short stub and the spill path, and SHALL leave message structure intact.

#### Scenario: Prune only
- **WHEN** the estimate passes the prune threshold and pruning brings it under the compaction budget
- **THEN** no summarization runs and the turn continues

#### Scenario: Stubbed result recoverable
- **WHEN** the model needs a pruned result again
- **THEN** the stub names the spill path and the model can read it

### Requirement: Compaction runs on an absolute token budget

The system SHALL compact when the estimated wire size exceeds 60,000 tokens by default, or when the user asks, and SHALL NOT compact while a tool result is pending.

#### Scenario: Budget reached at a turn boundary
- **WHEN** a turn completes and the estimate still exceeds the configured budget after pruning
- **THEN** compaction runs before the next request is built

#### Scenario: Budget reached mid-loop
- **WHEN** the estimate exceeds the budget while the model is still calling tools
- **THEN** compaction is deferred until the current turn completes

### Requirement: Compaction preserves recent context verbatim

The system SHALL keep the system prompt and the last four user/assistant exchanges unmodified, including their reasoning content and tool calls, together with the tool results those exchanges reference.

#### Scenario: Recent tool use survives
- **WHEN** compaction runs and the last exchange used `read`
- **THEN** that call and its result appear unchanged in the compacted history

#### Scenario: Earlier turns summarized
- **WHEN** twelve older exchanges are summarized
- **THEN** they are replaced by a single system message and the four newest exchanges remain

### Requirement: Compaction is reported

The system SHALL replace the summarized range with one system message named `compacted`, SHALL report message and token counts before and after, and SHALL record the event in the session log.

#### Scenario: Visible result
- **WHEN** compaction finishes
- **THEN** the transcript shows the old and new message and token counts, including that the cache was reset

### Requirement: Mid-loop context overflow degrades gracefully

The system SHALL respond to `context_length_exceeded` during a tool loop by truncating older tool results, retrying the same request once, and ending the turn with an error if it fails again.

#### Scenario: Overflow after a large tool result
- **WHEN** the API rejects a mid-loop request as too long
- **THEN** older tool results are shortened, the request is retried once, and the turn continues on success

### Requirement: The prompt names the session directory

The system SHALL name the session directory's absolute path in the system prompt and SHALL say that commands start there.

#### Scenario: Named
- **WHEN** a session starts in `/work/shop`
- **THEN** the system prompt contains `/work/shop`
