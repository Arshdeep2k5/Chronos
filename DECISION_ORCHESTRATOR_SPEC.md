# Decision Orchestrator Engine Specification

The Decision Orchestrator Engine is a Layer 4 (Decision) subsystem. It serves as the sole source of `ChronosDecision` resolutions, evaluating state, risks, capacities, commitments, deadlines, and reflection logs to decide on recovery and warning actions.

---

## 1. Specification

### Consumes
*   `ChronosState` materialized models.
*   `RiskForecast` snapshots (from the Risk Engine).
*   `CapacityProfile` snapshots.
*   `CommitmentCandidate` lists.
*   `DeadlineCandidate` lists.
*   `ChronosReflection` records.

### Produces
*   `ChronosDecision` entities containing a serialized JSON block in the `outcome_evaluation` property carrying the decision type, arbitration logic, and provenance list.
*   `DecisionResolved` event streams published onto the `EventBus`.

---

## 2. Decision Types

*   `NoAction`: System is steady; no alerts needed.
*   `Notify`: Emit a notification about approaching deadlines or minor decay.
*   `SuggestRecoveryPlan`: Recommend a timeline adjust or recovery workflow for a stalled project.
*   `SuggestWorkspaceRestore`: Re-activate focus layout if a session was closed via inactivity timeout.
*   `EscalateIntervention`: Immediate alert when project failure probability is critical.
*   `SuppressIntervention`: Keep notifications silent to prevent cognitive fatigue if burnout risk is high.

---

## 3. Decision Arbitration Logic (Factual & AI-Free)

### 3.1 Silence vs. Interruption Cost Evaluation
*   *Silence Cost:* Rises if critical project risks exist but are unaddressed.
*   *Interruption Cost:* Rises if user is highly fatigued (`burnout_risk > 0.8`).
*   *Arbitration:* If `burnout_risk > 0.8`, any standard `Notify` or `SuggestRecoveryPlan` is mapped to `SuppressIntervention` to preserve user attention.

### 3.2 Workspace Restore Rule
*   *Logic:* If the last session closed with an inactivity timeout (Interrupted Session), trigger `SuggestWorkspaceRestore`.

### 3.3 Critical Risk Escalation
*   *Logic:* If any project failure probability exceeds `0.75`, trigger `EscalateIntervention` immediately.

---

## 4. Replay safety
*   The orchestrator runs strictly deterministic math. Given identical input matrices (risk, capacity, commitments, sessions), it resolves to the exact same `ChronosDecision` output. No LLMs or Gemini APIs are used.
