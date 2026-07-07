# Reflection Engine Specification

The Reflection Engine is a Layer 3 (Reasoning) subsystem. It performs deterministic, evidence-based interpretation of `ChronosState`, active `CognitiveSession` history, and `EntityGraph` details to identify trends, context blocks, and focus states.

---

## 1. Specification

### Consumes
*   `ChronosState` materialized payloads.
*   `CognitiveSession` lists (Session history).
*   `EntityGraph` resolved entities.

### Produces
*   `ChronosReflection`: Containing a serialized JSON block in the `outcome_evaluation` property carrying the reasoning evidence, confidence ratings, and explanation.

---

## 2. Capabilities & Reasoning Rules

### 2.1 Stalled Projects Detection
*   **Rule:** If a `Project` entity exists in the graph, but no active `CognitiveSession` has been linked to its repository ID for $> 24$ hours, the project is classified as "stalled".
*   **Evidence:** `"Project <name> has not been modified since <timestamp>"`

### 2.2 Dormant Commitments Detection
*   **Rule:** Tracks if open commitments (or entities in "dormant" state classifications) remain untouched for $> 2$ hours while other active sessions are running.
*   **Evidence:** `"Commitment/Entity <id> remains open but has no recent focus activity"`

### 2.3 Context Drift Detection
*   **Rule:** Evaluates if consecutive `CognitiveSession`s switched to different repositories/projects.
*   **Evidence:** `"Context drifted from Repository <repo_A> to <repo_B> in consecutive sessions"`

### 2.4 Interrupted Sessions Detection
*   **Rule:** Detects if a session was closed due to inactivity timeout (rather than explicit shutdown events).
*   **Evidence:** `"Session <id> was closed due to inactivity gap of <seconds>"`

### 2.5 Active Focus Areas Detection
*   **Rule:** Analyzes which entity has the highest density of events in the active `CognitiveSession`.
*   **Evidence:** `"User focus is concentrated on File <path>"`

---

## 3. Serialization Contract
Since `ChronosReflection` from the frozen kernel has a generic `outcome_evaluation` string field, the detailed reflection payload is serialized to JSON:
```json
{
  "reflection_id": "uuid-1111",
  "timestamp": "2026-06-28T12:00:00Z",
  "confidence": 90,
  "evidence": [
    "Project Chronos has not been modified since 2026-06-27"
  ],
  "provenance_ids": ["sess-1234"],
  "explanation": "Project Chronos is stalled."
}
```
