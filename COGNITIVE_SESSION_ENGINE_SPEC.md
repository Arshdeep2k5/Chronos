# Cognitive Session Engine Specification

The Cognitive Session Engine is a Layer 2 (Memory) subsystem. It monitors the influx of raw events and resolved knowledge entities, grouping them into contiguous blocks of human attention representing `CognitiveSession` models.

---

## 1. Specification

### Consumes
*   `ChronosEvent` stream.
*   `KnowledgeEntity` inputs and resolved state from the `EntityGraph`.
*   Replayed events from `EventStore` for deterministic reconstruction.

### Produces
*   `CognitiveSession` structures.
*   Session lifecycle notifications:
    *   `SessionCreated`: Fired upon detection of new focal activity.
    *   `SessionUpdated`: Fired as events enrich the active session.
    *   `SessionClosed`: Fired when inactivity timeout breaches or explicit termination is received.
    *   `SessionResumed`: Fired when a new session is linked back to a recently closed session.

---

## 2. Session Attributes
Each session contains:
*   `session_id`: Unique identifier (UUID).
*   `start_timestamp`: Time of the first event in the session.
*   `end_timestamp`: Time of the last event in the session.
*   `duration`: Elapsed seconds between start and end.
*   `confidence`: Probability score of the session coherence.
*   `entity_ids`: Links to related knowledge graph nodes.
*   `repository_ids`: Links to Repository entities.
*   `artifact_ids`: Links to Artifact entities.
*   `project_ids`: Links to Project entities.
*   `source_event_ids`: Atomic events contributing to this session.
*   `resumes_session_id`: Linked preceding session (resumption chain).

---

## 3. Boundary & Continuation Rules

### Session Start Triggers
1.  **Activity emergence**: Any event occurs after a period of silence exceeding the inactivity window.
2.  **Repo/Branch Switch**: Explicit checkout or repo discovered event while inactive.
3.  **Density trigger**: Influx of $\ge 3$ events within 60 seconds.

### Session End Triggers
1.  **Explicit termination**: Lock screen, sleep, or shutdown events.
2.  **Extended inactivity**: Time delta between events exceeds the configurable timeout threshold (default: 900 seconds / 15 minutes).

### Resumption & Linkage
*   If inactivity time $T$ satisfies `threshold < T < 4 * threshold`, the new session will link to the previous session using the `resumes_session_id` field.
*   If $T \ge 4 * threshold$, it is treated as a clean restart (no resumes link).

---

## 4. Replay Semantics
The Cognitive Session Engine is fully deterministic. Given the exact same sequential array of `ChronosEvent`s:
1.  The session boundary detector applies thresholds in temporal order.
2.  Sessions are closed and opened at the exact same indices.
3.  Calculated duration, entity linkages, and confidence ratings must align identically on repeated runs.
