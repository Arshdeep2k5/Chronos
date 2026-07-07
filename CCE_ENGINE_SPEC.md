# Context Continuation Engine Specification

The Context Continuation Engine (CCE) is a Layer 4 (Execution) subsystem. It converts incoming `ChronosDecision` instructions into concrete, actionable `ChronosAction` plans such as Workspace Restores, Recovery Plans, or Continuation Plans.

---

## 1. Specification

### Consumes
*   `ChronosDecision` entity.
*   `ChronosState` materialized snapshot.
*   `CognitiveSession` history (Session Projection).
*   `CommitmentCandidate` lists.
*   `DeadlineCandidate` lists.
*   `RiskForecast` snapshots.

### Produces
*   `ChronosAction` entities (which carry the continuation plan details in the `payload` property).
*   `ContinuationPlanResolved`, `RecoveryPlanResolved`, and `WorkspaceRestoreRequested` event streams published onto the `EventBus`.

---

## 2. Action Plan payload schemas

### 2.1 WorkspaceRestoreRequest
*   *Triggers on:* `SuggestWorkspaceRestore` decision type.
*   *Payload structure:*
    ```json
    {
      "restore_target_session_id": "sess-123",
      "files_to_reopen": ["src/lib.rs"],
      "explanation": "Suggest restoring workspace focus layout after inactivity gap."
    }
    ```

### 2.2 RecoveryPlan
*   *Triggers on:* `SuggestRecoveryPlan` decision type.
*   *Payload structure:*
    ```json
    {
      "project_recovery_trajectories": {
        "proj-123": "stalled-recovery"
      },
      "dormant_project_reentry_points": ["src/lib.rs"],
      "recommended_next_action": "Resume development on stalled commitments"
    }
    ```

### 2.3 ContinuationPlan
*   *Triggers on:* `Notify` or `EscalateIntervention` decision types.
*   *Payload structure:*
    ```json
    {
      "digest": "Approaching milestones noted; sending routine status digest.",
      "recommended_next_action": "Review close deadlines"
    }
    ```

---

## 3. Boundary & Execution Logic
*   **Execution Mapping:** CCE translates the decision model directly to the appropriate action layout without fuzzy code paths.
*   **Replay Determinism:** Given identical decisions and session state references, CCE produces identical action structures, file paths, and metadata. No LLMs or Gemini APIs are used.
