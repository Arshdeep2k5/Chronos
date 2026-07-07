# DATABASE_ARCHITECTURE.md
*Authoritative Database Footprint & Storage Architecture Specification*

---

## 1. The Multi-Database Duality (Why Multiple DBs Exist)

Chronos is in transition from a relational architecture (MVP) to a modular event-sourced architecture (PCOS). This transition has left three independent database files on disk:

```
                      [Digital Environment]
                               │
               ┌───────────────┴───────────────┐
               ▼                               ▼
      [Tauri Monolith]                 [chronos-daemon]
               │                               │
       ┌───────┴───────┐                       │
       ▼               ▼                       ▼
  Relational       Ephemeral             Event-Sourced
  chronos.db  chronos_telemetry.db      chronos_events.db
```

### 1.1 Relational Store (`chronos.db`)
*   **Ownership**: Legacy Tauri App (`src-tauri/src/db.rs`).
*   **Authoritative For**: Python worker parameters, deadlines configurations, autonomous research logs, and historical relational commitments.
*   **Access Pattern**: Read/write via SQLite connection handles.

### 1.2 Telemetry Cache (`chronos_telemetry.db`)
*   **Ownership**: Legacy Tauri App (`src-tauri/src/db.rs`).
*   **Authoritative For**: Ephemeral browser sessions, raw searches, window focus transitions, and active editor tab snapshots.
*   **Access Pattern**: Append-only log.

### 1.3 Unified Event Store (`chronos_events.db`)
*   **Ownership**: Modular PCOS Daemon (`chronos-store-sqlite`).
*   **Authoritative For**: **All system ground truth**. The single source of truth for the core cognitive state loop.
*   **Access Pattern**: Append-only for incoming perception/action events, with read operations limited to boot-time warming replays.

---

## 2. Event Sourcing Strategy & Schemas

### 2.1 The Event Table Schema
All observations, intentions, decisions, and execution metrics are serialized as raw JSON payloads inside the `chronos_events` table of `chronos_events.db`:

```sql
CREATE TABLE IF NOT EXISTS chronos_events (
    id             TEXT PRIMARY KEY,
    timestamp      TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    event_type     TEXT NOT NULL,
    source         TEXT NOT NULL,
    payload        TEXT NOT NULL
);
```

### 2.2 Ephemeral Alerts Indexing
System warnings are indexed separately inside the `chronos_alerts` table:

```sql
CREATE TABLE IF NOT EXISTS chronos_alerts (
    alert_id       TEXT PRIMARY KEY,
    event_id       TEXT NOT NULL,
    timestamp      TEXT NOT NULL,
    severity       TEXT NOT NULL,
    component      TEXT NOT NULL,
    event_type     TEXT NOT NULL,
    message        TEXT NOT NULL,
    metadata       TEXT NOT NULL,
    FOREIGN KEY(event_id) REFERENCES chronos_events(id) ON DELETE CASCADE
);
```

---

## 3. Storage Optimization & Indexes

To ensure high throughput and prevent write locks in SQLite, the databases configure the following SQL primitives:
1.  **WAL Mode**: `PRAGMA journal_mode = WAL;` allows concurrent readers to access state while the pipeline loop appends new events.
2.  **Performance Indices**:
    ```sql
    CREATE INDEX IF NOT EXISTS idx_events_timestamp ON chronos_events(timestamp);
    CREATE INDEX IF NOT EXISTS idx_events_event_type ON chronos_events(event_type);
    CREATE INDEX IF NOT EXISTS idx_alerts_severity ON chronos_alerts(severity);
    ```

---

## 4. Authoritative Source of Truth Matrix

| Data Type | Primary Table | Database | Authoritative Subsystem |
| :--- | :--- | :--- | :--- |
| **Observation Event Log** | `chronos_events` | `chronos_events.db` | `chronos-store-sqlite` |
| **Active Commitments** | `commitments` | `chronos.db` | `chronos-reasoning-commitments` |
| **Workspace Snapshots** | `workspace_snapshots` | `chronos_telemetry.db` | `chronos-memory-sessions` |
| **Research Briefs** | `autonomous_research_briefs`| `chronos.db` | Python worker (`arc_crawler.py`) |
| **Performance Anomaly Alerts**| `chronos_alerts` | `chronos_events.db` | `chronos-store-sqlite` |

---
