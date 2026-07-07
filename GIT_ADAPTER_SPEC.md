# Chronos Git Adapter Specification

The Git Adapter is a Perception Layer subsystem. Its sole responsibility is to observe local Git repositories, extract repository metadata/events, normalize them into `ChronosEvent` formats, and publish them onto the `Cognitive Bus`.

---

## 1. Specification

### Consumes
*   **Git Repository States**: Read from the local `.git` directory structure (specifically `.git/logs/HEAD`, `.git/HEAD`, and `.git/config`).
*   **Metadata**: Authorship, branch configurations, repository names, and commit histories.
*   **Polling/Filesystem Watcher**: Triggers on modifications to the repository metadata directory (`.git/`).

### Produces
*   `ChronosEvent` containing the normalized action. The event payload preserves the raw git commit hashes, committer details, and branch names.

### Capabilities
*   **Event Types Detected**:
    *   `GitRepositoryDiscovered`: Fired when a repository is added to the monitor list.
    *   `GitRepositoryOpened`: Fired when Chronos registers active work context inside the repo.
    *   `GitBranchSwitched`: Fired when checkout actions occur.
    *   `GitCommitCreated`: Fired on standard commits.
    *   `GitCommitAmended`: Fired when a commit is amended.
    *   `GitMergePerformed`: Fired on branch merge.
    *   `GitTagCreated`: Fired on tagging.
    *   `GitRepositoryModified`: Fired on generic updates (index changes).
    *   `GitRepositoryRemoved`: Fired if a repository is untracked.

### Dependencies
*   `chronos-core`: For constructing canonical events.
*   `chronos-bus`: For event publication.
*   `chronos-logging`: For structured debug tracing.
*   `serde_json` and `chrono`: For serialization and timestamps.

### Failure Modes
*   **Locked `.git` files**: Temporary lock files (`.git/index.lock`) might block filesystem reads. Resolved with simple retry limits.
*   **Corrupted Reflogs**: If reflogs are disabled in the repo, the adapter falls back to polling the latest `.git/refs/heads/` tip or running git commands.

---

## 2. Reflog Parsing & Event Mapping

The `.git/logs/HEAD` reflog is parsed line-by-line:
```text
<old-sha> <new-sha> <committer-name> <committer-email> <timestamp> <tz> <message>
```

### Normalization Logic

| Reflog Message Match | Normalized Event Type | Extracted Event Payload |
| :--- | :--- | :--- |
| `checkout: moving from <x> to <y>` | `GitBranchSwitched` | `from_branch`, `to_branch`, `new_sha` |
| `commit: <msg>` | `GitCommitCreated` | `sha`, `message`, `author`, `author_email` |
| `commit (amend): <msg>` | `GitCommitAmended` | `sha`, `message`, `old_sha` |
| `merge <branch>: <msg>` | `GitMergePerformed` | `merged_branch`, `target_sha` |
| `tag: <msg>` | `GitTagCreated` | `tag_name`, `target_sha` |

---

## 3. Integration & Lifecycle

```text
[Local Git Repo]
       │ (.git/logs/HEAD changes)
       ▼
[GitRepositoryObserver] ──(Detects change)──► [GitEventNormalizer]
                                                    │
                                                    ▼ (Maps to ChronosEvent)
[EventBus] ◄──(Publishes event)─────────── [GitEventPublisher]
```

1.  **Registration**: On startup, the adapter registers its service descriptors and capability assertions in the `ServiceRegistry`.
2.  **Observation**: Spins up a background thread that monitors configured repository folders.
3.  **Publishing**: Normalizes operations and pushes them directly onto the `EventBus` without executing reasoning or mutating state.
