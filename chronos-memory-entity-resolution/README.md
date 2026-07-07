# chronos-memory-entity-resolution

A Layer 2 (Memory / Knowledge) subsystem. It converts raw, unstructured `ChronosEvent` logs into structured, canonical `KnowledgeEntity` nodes and links them inside an `EntityGraph`. Completely deterministic and AI-free.

## Specification

### Consumes
- `ChronosEvent` streams from the `EventBus`.
- `ChronosEvent` replays from the `EventStore`.

### Produces
- `KnowledgeEntity` structures inside `EntityGraph`.
- Side-effect events:
  - `EntityCreated`
  - `EntityUpdated`
  - `EntityLinked`

### Capabilities
- **Factual, Evidence-Based Resolution**: Graph resolution relies strictly on deterministic rules, with zero fuzzy inference, LLM calls, or heuristic guess-work.
- **Durable Replay Reconstruction**: The entire graph projection can be reconstructed deterministically by running a query stream over the historical `EventStore`.
- **Confidence Tracking**: Nodes carry confidence flags indicating the certainty of their structural resolution.

### Dependencies
- `chronos-core`: Shared types.
- `chronos-bus`: Notification transport.
- `chronos-store`: Historical event storage.

### Failure Modes
- **Orphaned Entities**: If a file event is received before repository discovery, the file cannot be immediately linked (handled by resolving parent repository retroactively).
- **Out of Order Replays**: Addressed by strictly sorting events by timestamp prior to rule application.

### Resolution Rules
- `ResolveGitRepositoryRule`: Reads repository discovery events and git details. Resolves `Repository` and `Branch` nodes.
- `ResolveFileRule`: Detects file references across commits, resolving `File` and linking them to parent `Repository` nodes. Repeated references prompt promotion to `Artifact` entities.

### Entity Types
- `Project`
- `Artifact`
- `Repository`
- `File`
- `Branch`
- `Workspace`
- `Commitment` (stub placeholder)

### Graph Model
```text
(Workspace) ──contains──► (Project)
                            │
                            └──tracks──► (Repository) ──has_branch──► (Branch)
                                           │
                                           └──owns_file──► (File) ──represents──► (Artifact)
```
