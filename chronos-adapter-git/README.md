# chronos-adapter-git

A Perception Layer adapter for the Chronos PCOS. It monitors local Git repositories and streams reflog changes onto the Cognitive Bus.

## Documentation

### Consumes
- **Git HEAD logs**: Read from `.git/logs/HEAD`.
- **HEAD ref**: Read from `.git/HEAD` to detect checkout branches.

### Produces
- **ChronosEvent**: Structured payloads mapped to events (`GitCommitCreated`, `GitBranchSwitched`, `GitMergePerformed`, `GitTagCreated`, `GitRepositoryModified`).

### Capabilities
- **Non-blocking Polling**: Iterates through watched repositories without hogging execution threads.
- **Durable Offset Tracking**: Remembers the line position in log streams to avoid duplicate event emission across process restarts.
- **Factual Mapping**: Extracts exact SHAs, names, emails, and commit messages without applying reasoning, interpretation, or AI operations.

### Dependencies
- `chronos-core`: The base COM data models.
- `chronos-bus`: The pub/sub transport.
- `chronos-registry`: Capability advertisement.
- `chronos-logging`: Observability hooks.
- `chronos-config`: Access to monitored repository path settings.

### Failure Modes
- **Access Violation**: If permissions are denied, logs a warning and retries on next poll pass.
- **Missing Reflogs**: If a repository doesn't have reflogs enabled, the adapter logs the issue and waits for standard ref creations.

### Observed Git Operations & ChronosEvent Mapping

| Git Reflog Event Example | Extracted Message | ChronosEvent Type |
| :--- | :--- | :--- |
| `commit: Implement git adapter` | `Implement git adapter` | `GitCommitCreated` |
| `checkout: moving from master to dev` | `moving from master to dev` | `GitBranchSwitched` |
| `commit (amend): Fix bug` | `Fix bug` | `GitCommitAmended` |
| `merge dev: Merge branch 'dev'` | `Merge branch 'dev'` | `GitMergePerformed` |
| `tag: v1.0` | `v1.0` | `GitTagCreated` |
