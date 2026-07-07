# chronos-reasoning-risk

The Risk Analysis Engine is a Layer 3 (Reasoning) subsystem. It calculates project failure probabilities, context decay trajectories, and intervention urgency parameters by combining active capacity profiles, deadline constraints, and historical session activity.

## Specification

### Consumes
- `ChronosState` materialized snapshots.
- `CognitiveSession` history (Session Projection).
- `CommitmentCandidate` lists.
- `DeadlineCandidate` lists.
- `CapacityProfile` snapshots resolved by the PCM engine.

### Produces
- `RiskForecast` event streams published onto the `EventBus` under the `RiskForecastResolved` event type.

### Capabilities
- **Project Failure Probability**: Triggers high failure probability forecasts when capacity is low, burnout is high, or deadlines are close.
- **Context Decay Trajectory**: Measures focus decay relative to hours elapsed since a project's last active session.
- **Intervention Urgency**: Signals alert thresholds when failure rates or fatigue levels peak.

### Dependencies
- `chronos-core`: Schema type definitions.
- `chronos-memory-sessions`: Active session history.
- `chronos-reasoning-commitments`: Commitment tracking.
- `chronos-reasoning-dde`: Deadline constraints.
- `chronos-reasoning-pcm`: Personal capacity indicators.

### Failure Modes
- **Date discrepancies**: Handled by falling back to total context decay if logs are missing.
