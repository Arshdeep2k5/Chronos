# chronos-memory-sessions

The Cognitive Session Engine is a Layer 2 (Memory) subsystem. It groups raw events and knowledge entities into deterministic, chronological Cognitive Sessions.

## Specification

### Consumes
- `ChronosEvent` streams from the `EventBus`.
- `KnowledgeEntity` inputs and resolved state from the `EntityGraph`.
- Replayed events from `EventStore` for deterministic reconstruction.

### Produces
- `CognitiveSession` structures.
- Session lifecycle notifications:
  - `SessionCreated`
  - `SessionUpdated`
  - `SessionClosed`
  - `SessionResumed`

### Capabilities
- **Deterministic Boundary Detection**: Automatically groups events into focus sessions without fuzzy inference or AI intervention.
- **Durable Replays**: Re-evaluating event logs produces identical session offsets, limits, and relationships.
- **Context Chaining**: Preserves session continuity histories by linking consecutive active blocks using the `resumes_session_id` reference.

### Dependencies
- `chronos-core`: The base object schema definitions.
- `chronos-memory-entity-resolution`: Access to resolved project and repository entities.

### Failure Modes
- **Buffer Exhaustion**: Extremely fast event storms may flood boundary detectors (handled via temporal separation).
- **Out of Order Arrival**: Replay steps sort logs chronologically before calculations.

### Boundary Detection Rules
- **Start Triggers**: Emergence of activity after silence, or explicit branch switches/checkout operations.
- **End Triggers**: Timeout checks (exceeding configurable inactivity windows) or explicit shutdown signals.

### Continuation Rules
- Gaps shorter than the inactivity threshold (default: 15 minutes) extend the current session.
- Gaps between 1x and 4x the threshold result in a new session linked via `resumes_session_id` to the previous one. Gaps larger than 4x start a clean session with no resumes link.
