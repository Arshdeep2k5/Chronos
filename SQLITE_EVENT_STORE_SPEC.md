# SQLite Event Store Specification

This document defines the schema design, capabilities, and performance attributes of the `SQLiteEventStore` implementation for the Chronos PCOS.

---

## 1. Specification

### Consumes
*   `ChronosEvent`: Serialized into SQLite rows.
*   Durable writes: Persisted to a local SQLite database.

### Produces
*   `ChronosEvent`: Deserialized from stored rows.
*   Query results: Replay timelines, latest event counts, and single event fetches.

### Capabilities
*   **Durable Persistence**: Preserves logs across restarts and system crashes.
*   **Index-Optimized Queries**: Leverages composite indices for high-performance timestamp and event type lookups.
*   **Thread Safety**: Uses `Arc<Mutex<Connection>>` or thread-safe pool abstractions to serialize access safely in multi-threaded Tokio runtimes.
*   **Write-Ahead Logging (WAL)**: Enabled by default to maximize write throughout.

### Dependencies
*   `rusqlite`: The Rust interface to SQLite.
*   `serde_json`: Handles payload serialization/deserialization.
*   `chrono`: For timezone-aware event timestamps.

### Failure Modes
*   `StoreError::AppendError`: Database disk full, lock contention, or foreign key violations.
*   `StoreError::RetrieveError`: Invalid JSON formatting in the database or schema mismatch.
*   `StoreError::Unavailable`: Missing/inaccessible database file.

---

## 2. Schema Design

```sql
CREATE TABLE IF NOT EXISTS chronos_events (
    id             TEXT PRIMARY KEY,
    timestamp      TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    event_type     TEXT NOT NULL,
    source         TEXT NOT NULL,
    payload        TEXT NOT NULL
);

-- Indices for rapid timeline replays and filters
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON chronos_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON chronos_events(event_type);
```

### Constraints
1.  **Append-Only**: No `UPDATE` or `DELETE` operations are exposed on the `EventStore` interface.
2.  **Immutability**: Raw `ChronosEvent` payloads must never be altered once persisted.
3.  **Strict Ordering**: Events retrieved via `stream`, `replay`, or `latest` must be sorted chronologically by the `timestamp` field.

---

## 3. Migration Strategy
*   On initialization, the `SQLiteEventStore` verifies the presence of the `chronos_events` table.
*   If not found, it runs the creation DDL script automatically (Auto-Migration).
*   Any future schema upgrades will detect the `schema_version` of historical records and convert them on-the-fly or execute migration DDL steps.
