# Personal Capacity Model Engine Specification

The Personal Capacity Model (PCM) Engine is a Layer 3 (Reasoning) subsystem. It estimates available execution capacity, focus levels, throughput, focus stability, and burnout risk metrics from `ChronosState`, active session history, resolved commitments, and deadline candidates.

---

## 1. Specification

### Consumes
*   `ChronosState` materialized payloads.
*   `CognitiveSession` history (Session Projection).
*   `CommitmentCandidate` lists.
*   `DeadlineCandidate` lists.
*   `ChronosEvent` streams from the `EventBus` or `EventStore`.

### Produces
*   `CapacityProfile` event streams published onto the `EventBus` under the `CapacityProfileResolved` event type.

---

## 2. Capacity Profile Schema

Every generated capacity profile contains:
```json
{
  "capacity_score": 0.85,
  "focus_score": 0.90,
  "throughput_score": 0.75,
  "stability_score": 0.80,
  "burnout_risk": 0.15,
  "confidence": 0.95,
  "provenance_ids": ["sess-1", "sess-2", "com-1"]
}
```

---

## 3. Capacity & Burnout Rules (Deterministic & AI-Free)

1.  **Focus Score**:
    *   *Logic:* Measures average session duration. Scaled against a maximum focus threshold of 1 hour (3600s).
    *   *Formula:* `min(1.0, average_session_duration / 3600.0)`.
2.  **Throughput Score**:
    *   *Logic:* Evaluates resolved or candidate commitment density. Scaled against a reference baseline of 10 commitments.
    *   *Formula:* `min(1.0, commitments_created / 10.0)`.
3.  **Stability Score**:
    *   *Logic:* Analyzes context switching rates across projects and repositories.
    *   *Formula:* `1.0 - min(1.0, context_switching_frequency / 5.0)`.
4.  **Burnout Risk**:
    *   *Logic:* Identifies long work intervals or rapid session restarts. If sessions per day exceeds 8 or average session duration exceeds 2 hours (7200s), risk is set to `0.85`, else `0.10`.
5.  **Capacity Score**:
    *   *Formula:* `(focus_score + throughput_score + stability_score) / 3.0 * (1.0 - burnout_risk)`.

---

## 4. Replay & Persistence Design
*   Like other reasoning engines, all calculation steps are strictly math-based and deterministic. Replaying the identical session logs and state records always produces identical Capacity Scores and Burnout Risks. No LLMs or Gemini APIs are used.
