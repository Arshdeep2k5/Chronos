# chronos-reasoning-dde

The Deadline Discovery Engine (DDE) is a Layer 3 (Reasoning) subsystem. It infers explicit, inferred, and repository-derived deadlines from `ChronosState`, resolved graph entities, active session density patterns, and event descriptions.

## Specification

### Consumes
- `ChronosState` materialized payloads.
- `CommitmentCandidate` items resolved by the commitment engine.
- `CognitiveSession` history (Session Projection).
- `EntityGraph` resolved entities.
- `ChronosEvent` streams from the `EventBus` or `EventStore`.

### Produces
- `DeadlineCandidate` event streams published onto the `EventBus` under the `DeadlineCandidateResolved` event type.

### Capabilities
- **Explicit Date Regex Scanning**: Scans logs for patterns matching `(?i)(?:due|deadline|milestone|release)\s+(\d{4}-\d{2}-\d{2})`.
- **Inferred Acceleration Detection**: Resolves near-term deadline predictions when session focus density spikes ($ge 3$ sessions in $< 24$ hours).
- **Milestone Detection**: Identifies release versions (`v1.0`), release branch names, or milestone tags to infer deadlines.

### Dependencies
- `chronos-core`: Schema contract parameters.
- `chronos-memory-entity-resolution`: Entity properties.
- `chronos-memory-sessions`: Active session streams.
- `chronos-reasoning-commitments`: Commitment tracking.

### Failure Modes
- **Date String Parse anomalies**: Handled gracefully by dropping invalid format strings.
