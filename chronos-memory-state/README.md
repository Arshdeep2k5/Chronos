# chronos-memory-state

The State Projector is a Layer 2 (Memory) subsystem. It materializes `ChronosState` from the structured outputs of the `EntityGraph` and active `CognitiveSession` streams.

## Specification

### Consumes
- `CognitiveSession` streams / projections.
- `EntityGraph` resolved entities and relationships.
- `ChronosEvent` streams from the `EventBus` or `EventStore`.

### Produces
- `ChronosState` structures.

### Capabilities
- **Focal Entity Classification**: Categorizes entities into active, dormant, archived, and incomplete segments based on access times, parent linkages, and focus sessions.
- **Reliable Confidence Aggregation**: Automatically computes mathematical averages of resolved graph entity confidence levels.
- **Freshness Verification**: Dates projections with UTC timestamps to check for context age.

### Dependencies
- `chronos-core`: Schema models.
- `chronos-memory-entity-resolution`: Access to resolved project and file entities.
- `chronos-memory-sessions`: Active session streams.

### Failure Modes
- **Orphaned File References**: Unmapped file paths are classified as "incomplete" entities until linked to a parent repository node.
- **Stale State Processing**: Prevented by verifying and logging the freshness timestamp before consumption by higher reasoning layers.
