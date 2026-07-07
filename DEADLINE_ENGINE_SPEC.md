# Deadline Discovery Engine Specification

The Deadline Discovery Engine (DDE) is a Layer 3 (Reasoning) subsystem. It infers explicit, inferred, and repository-derived deadlines from `ChronosState`, resolved graph entities, active session density patterns, and event descriptions.

---

## 1. Specification

### Consumes
*   `ChronosState` materialized payloads.
*   `CommitmentCandidate` items resolved by the commitment engine.
*   `CognitiveSession` history (Session Projection).
*   `EntityGraph` resolved entities.
*   `ChronosEvent` streams from the `EventBus` or `EventStore`.

### Produces
*   `DeadlineCandidate` event streams published onto the `EventBus` under the `DeadlineCandidateResolved` event type.

---

## 2. Deadline Candidate Schema

```json
{
  "deadline_id": "uuid-5555",
  "commitment_id": "uuid-9999",
  "target_date": "2026-12-01T00:00:00Z",
  "confidence": 0.95,
  "evidence_ids": ["event-1", "event-2"],
  "source_type": "Explicit",
  "created_at": "2026-06-28T12:00:00Z",
  "updated_at": "2026-06-28T12:00:00Z"
}
```

### Source Types
1.  **Explicit**: Parsed from string matches like `"due 2026-12-01"`, `"milestone 2026-12-01"`, or `"submission deadline"`.
2.  **Inferred**: Parsed from temporal focus patterns like a sudden acceleration in session frequency (e.g. 3 sessions in 24 hours triggers a near-term deadline candidate).
3.  **RepositoryDerived**: Extracted from Git tags matching release numbers (`v1.0`), release branches, or version milestones.

---

## 3. Boundary & Analysis Rules (Deterministic & AI-Free)

### Explicit Date Regex Parsing
DDE runs a regular expression scanner against all commit messages and discovery payloads:
```text
(?i)(?:due|deadline|milestone|release)\s+(\d{4}-\d{2}-\d{2})
```
If a match is found, a date is parsed and resolved with `confidence = 1.0`.

### Inferred Acceleration Detection
If a commitment accumulates $\ge 3$ sessions in a 24-hour period, a near-term deadline of `now + 2 days` is inferred with `confidence = 0.8`.

### Repository Milestone Extraction
If a `GitTagCreated` event matches a release tag format (`v\d+\.\d+`), a repository deadline candidate is generated for `tag_timestamp + 1 day` with `confidence = 0.9`.

---

## 4. Replay Determinism
All DDE rules are completely state-free and deterministic. Replaying the identical event store history will generate identical deadline candidate IDs, dates, and classifications. No LLMs or Gemini APIs are used.
