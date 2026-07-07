# chronos-decision-orchestrator

The Decision Orchestrator Engine is a Layer 4 (Decision) subsystem. It serves as the sole source of `ChronosDecision` resolutions, evaluating state, risks, capacities, commitments, deadlines, and reflection logs to decide on recovery and warning actions.

## Specification

### Consumes
- `ChronosState` materialized models.
- `RiskForecast` snapshots (from the Risk Engine).
- `CapacityProfile` snapshots.
- `CommitmentCandidate` lists.
- `DeadlineCandidate` lists.
- `ChronosReflection` records.

### Produces
- `ChronosDecision` entities containing a serialized JSON block in the `outcome_evaluation` property carrying the decision type, arbitration logic, and provenance list.
- `DecisionResolved` event streams published onto the `EventBus`.

### Decision Types
- `NoAction`: System is steady; no alerts needed.
- `Notify`: Emit a notification about approaching deadlines or minor decay.
- `SuggestRecoveryPlan`: Recommend a timeline adjust or recovery workflow for a stalled project.
- `SuggestWorkspaceRestore`: Re-activate focus layout if a session was closed via inactivity timeout.
- `EscalateIntervention`: Immediate alert when project failure probability is critical.
- `SuppressIntervention`: Keep notifications silent to prevent cognitive fatigue if burnout risk is high.

### Dependencies
- `chronos-core`: Schema definitions.
- `chronos-memory-sessions`: Active session history.
- `chronos-reasoning-commitments`: Commitment tracking.
- `chronos-reasoning-dde`: Deadline candidates.
- `chronos-reasoning-pcm`: Personal capacity indicators.
- `chronos-reasoning-risk`: Risk forecasts.

### Failure Modes
- **Serialization errors**: Handled gracefully by returning default payload formats.
