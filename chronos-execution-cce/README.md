# chronos-execution-cce

The Context Continuation Engine (CCE) is a Layer 4 (Execution) subsystem. It converts incoming `ChronosDecision` instructions into concrete, actionable `ChronosAction` plans such as Workspace Restores, Recovery Plans, or Continuation Plans.

## Specification

### Consumes
- `ChronosDecision` entity.
- `ChronosState` materialized snapshot.
- `CognitiveSession` history (Session Projection).
- `CommitmentCandidate` lists.
- `DeadlineCandidate` lists.
- `RiskForecast` snapshots.

### Produces
- `ChronosAction` entities (which carry the continuation plan details in the `payload` property).
- `ContinuationPlanResolved`, `RecoveryPlanResolved`, and `WorkspaceRestoreRequested` event streams published onto the `EventBus`.

### Capabilities
- **Workspace Restore Request**: Formulates target workspace configurations, files to reopen, and session IDs to recover.
- **Recovery Plan Calculation**: Packages re-entry files and project trajectories for inactive workloads.
- **Continuation Plan Generation**: Prepares routine timeline warnings and next step recommendations.

### Dependencies
- `chronos-core`: Action and Decision schemas.
- `chronos-memory-sessions`: Active session history.
- `chronos-reasoning-commitments`: Commitment metadata.
- `chronos-reasoning-dde`: Deadline candidates.
- `chronos-reasoning-risk`: Risk forecasts.

### Failure Modes
- **Decision mismatch**: Returns `None` if the decision type does not warrant action execution.
