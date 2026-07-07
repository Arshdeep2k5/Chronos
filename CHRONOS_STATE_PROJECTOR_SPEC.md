# Chronos State Projector Specification

The Chronos State Projector is a Layer 2 (Memory) subsystem. It materializes `ChronosState` from the structured outputs of the `EntityGraph` and active `CognitiveSession` streams. It forms the canonical world model consumed by reasoning, decision, and presentation layers.

---

## 1. Specification

### Consumes
*   `CognitiveSession` stream / projection.
*   `EntityGraph` resolved entities and relationships.
*   `ChronosEvent` streams from the `EventBus` or `EventStore`.

### Produces
*   `ChronosState`: Canonical runtime snapshot containing schema-versioned state properties, active intents, and metadata.

### Capabilities
*   **Active classification**: Entities mutated or read during the active `CognitiveSession`.
*   **Dormant classification**: Unresolved entities (e.g. projects, files) outside the active session that remain incomplete.
*   **Archived classification**: Factual objects linked to closed, inactive sessions that have decayed.
*   **Incomplete classification**: Entity nodes lacking requisite semantic links (e.g. a `File` not mapped to a `Repository`).
*   **Confidence Aggregation**: Calculates a mathematical rolling average of confidence indexes across all active entities.
*   **Freshness Timestamping**: Explicitly dates the materialized projection to detect context latency.

---

## 2. Structural Schema

The materialized state is stored inside the `payload` property of `ChronosState`:
```json
{
  "freshness_timestamp": "2026-06-28T12:00:00Z",
  "aggregated_confidence": 0.95,
  "active_session_id": "uuid-1234",
  "classifications": {
    "active_entity_ids": ["entity-1"],
    "dormant_entity_ids": ["entity-2"],
    "archived_entity_ids": ["entity-3"],
    "incomplete_entity_ids": ["entity-4"]
  },
  "provenance_event_ids": ["event-1", "event-2"]
}
```

---

## 3. Replay Determinism
Because `ChronosState` is resolved by passing the inputs sequentially through the projection engine:
*   Replaying the identical array of events and session boundaries will produce the exact same sequence of state version updates.
*   No fuzzy logic or dynamic AI values are allowed in the state projector, ensuring 100% auditability and state recovery from log files.
