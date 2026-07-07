# chronos-reasoning-pcm

The Personal Capacity Model (PCM) Engine is a Layer 3 (Reasoning) subsystem. It estimates available focus, stability, throughput, and burnout risk metrics from `ChronosState`, active session history, resolved commitments, and deadline candidates.

## Specification

### Consumes
- `ChronosState` materialized payloads.
- `CognitiveSession` history (Session Projection).
- `CommitmentCandidate` lists.
- `DeadlineCandidate` lists.
- `ChronosEvent` streams from the `EventBus` or `EventStore`.

### Produces
- `CapacityProfile` event streams published onto the `EventBus` under the `CapacityProfileResolved` event type.

### Capabilities
- **Focus Scoring**: Scales focus quality using average session durations.
- **Throughput Profiling**: Computes commitment output velocity.
- **Stability Analysis**: Monitors context-switching rates across repositories and projects.
- **Burnout Mitigation**: Pinpoints excessive workloads or session accelerations.

### Dependencies
- `chronos-core`: Schema type formats.
- `chronos-memory-sessions`: Active focus boundaries.
- `chronos-reasoning-commitments`: Commitment tracking.
- `chronos-reasoning-dde`: Deadline candidates.

### Failure Modes
- **Sparse History**: Settles back to baseline defaults to keep metrics reliable.
