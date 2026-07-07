# chronos-reasoning-reflection

The Reflection Engine is a Layer 3 (Reasoning) subsystem. It performs deterministic, evidence-based interpretation of `ChronosState`, active `CognitiveSession` history, and `EntityGraph` details to identify trends, context blocks, and focus states.

## Specification

### Consumes
- `ChronosState` materialized payloads.
- `CognitiveSession` lists (Session history).
- `EntityGraph` resolved entities.

### Produces
- `ChronosReflection` structures containing a serialized JSON block in the `outcome_evaluation` property carrying the reasoning evidence, confidence ratings, and explanation.

### Capabilities
- **Stalled Projects Detection**: Identifies inactive projects whose repositories have no linked focus session.
- **Interrupted Session Audits**: Pinpoints focus sessions closed due to inactivity timeout gaps rather than clean system shutdown triggers.
- **Active Focus Areas**: Pinpoints the repositories and artifacts capturing the highest user interaction levels.

### Dependencies
- `chronos-core`: Shared core data types.
- `chronos-memory-entity-resolution`: Entity structures.
- `chronos-memory-sessions`: Focus session boundaries.
- `chronos-memory-state`: Projected state payload formats.

### Failure Modes
- **State Serialization mismatch**: Dealt with by returning clear `ReflectionError` logs during parsing.
