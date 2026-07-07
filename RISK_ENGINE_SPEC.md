# Risk Analysis Engine Specification

The Risk Analysis Engine is a Layer 3 (Reasoning) subsystem. It calculates project failure probabilities, context decay trajectories, and intervention urgency parameters by combining active capacity profiles, deadline constraints, and historical session activity.

---

## 1. Specification

### Consumes
*   `ChronosState` materialized snapshots.
*   `CognitiveSession` history (Session Projection).
*   `CommitmentCandidate` lists.
*   `DeadlineCandidate` lists.
*   `CapacityProfile` snapshots resolved by the PCM engine.

### Produces
*   `RiskForecast` event streams published onto the `EventBus` under the `RiskForecastResolved` event type.

---

## 2. Risk Forecast Schema

Every generated risk forecast contains:
```json
{
  "project_failure_probabilities": {
    "proj-123": 0.35
  },
  "context_decay_trajectory": {
    "proj-123": 0.45
  },
  "intervention_urgency": 0.45,
  "confidence": 0.90,
  "provenance_ids": ["sess-1", "com-1", "dead-1"]
}
```

---

## 3. Calculation & Forecast Rules (Deterministic & AI-Free)

1.  **Project Failure Probability**:
    *   *Logic:* A project's failure probability rises if commitments have near deadlines ($< 3$ days), the user's workload throughput is low ($< 0.4$), or burnout risk is elevated ($> 0.7$).
    *   *Formula:* `0.10 + (1.0 - throughput_score) * 0.40 + (burnout_risk) * 0.30 + (if deadline < 3 days then 0.20 else 0.00)`. Capped between 0.0 and 1.0.
2.  **Context Decay Trajectory**:
    *   *Logic:* Measures decay based on time elapsed since the last active session referencing the project.
    *   *Formula:* `min(1.0, hours_since_last_session / 72.0)`.
3.  **Intervention Urgency**:
    *   *Logic:* Urgency is the maximum of the burnout risk and the highest project failure probability.
    *   *Formula:* `max(burnout_risk, max(project_failure_probabilities))`.

---

## 4. Replay Safety
*   The calculations are entirely stateless and mathematical. Replaying the identical session metrics and commitments always yields the exact same failure probability outputs, context decay rates, and intervention urgency parameters. No LLMs or Gemini APIs are used.
