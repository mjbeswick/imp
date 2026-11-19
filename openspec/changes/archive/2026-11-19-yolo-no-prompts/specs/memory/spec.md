# Spec Delta

## MODIFIED Requirements

### Requirement: Durable facts are captured at compaction

The system SHALL ask the summarizer to emit up to three durable facts, SHALL offer each as a `save` proposal rather than writing it, and in `yolo` SHALL save each without a prompt.

#### Scenario: Compaction finds a preference
- **WHEN** compaction runs over a conversation where the user stated a lasting preference
- **THEN** the interface offers to save it and writes nothing without approval

#### Scenario: Yolo
- **WHEN** compaction finds a durable fact in `yolo`
- **THEN** the fact is saved and no confirmation appears
