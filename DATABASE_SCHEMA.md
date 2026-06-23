# Database Design Document: Chronos Pilot (v1.0)

This document outlines the SQLite schema, table relations, indexes, and write configuration optimization strategies.

## 1. Engine Configuration (WAL Baseline)

To optimize local execution write throughput and concurrent reads, the database is opened with these PRAGMAs on boot:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 300000000; -- 300MB memory mapped file access
```

## 2. Table Schemas & Relational Mappings

```text
  ┌─────────────────┐             ┌─────────────────┐             ┌─────────────────┐
  │    projects     │◄───────────┼    project_state│             │   dead_letter   │
  └────────┬────────┘             └─────────────────┘             └─────────────────┘
           │
           ├───────────────────────────────┬───────────────────────────────┐
           ▼                               ▼                               ▼
  ┌─────────────────┐             ┌─────────────────┐             ┌─────────────────┐
  │  context_nodes  │◄────────────│   commitments   │◄────────────│ recovery_plans  │
  └────────┬────────┘             └────────┬────────┘             └─────────────────┘
           │                               │
           ▼                               ▼
  ┌─────────────────┐             ┌─────────────────┐
  │ context_events  │             │ project_actions │
  └─────────────────┘             └─────────────────┘
```

### 2.1 Core Project & Telemetry Tables

#### Table: `projects`
Stores the local project workspaces registry.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_name`: TEXT UNIQUE NOT NULL
* `status`: TEXT NOT NULL DEFAULT 'ACTIVE' (Constraints: `ACTIVE`, `ARCHIVED`)
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `project_state`
Caches the dynamic summary, next actions, and computed state coordinates.
* `project_id`: INTEGER PRIMARY KEY REFERENCES `projects(id)` ON DELETE CASCADE
* `current_summary`: TEXT NOT NULL
* `current_entry_point`: TEXT
* `next_action`: TEXT
* `confidence_score`: REAL NOT NULL DEFAULT 1.0
* `updated_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `context_nodes`
Tracks stateful physical and virtual context assets.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE SET NULL
* `entity_key`: TEXT UNIQUE NOT NULL (e.g., `FILE:/path/to/stripe.ts` or `DOMAIN:docs.stripe.com`)
* `entity_type`: TEXT NOT NULL (Constraints: `FILE`, `URL`, `DOCUMENT`, `RESEARCH_TOPIC`, `RESEARCH_SESSION`)
* `display_name`: TEXT NOT NULL
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `context_events`
The event-sourced ledger capturing interaction states.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `node_id`: INTEGER REFERENCES `context_nodes(id)` ON DELETE CASCADE
* `event_type`: TEXT NOT NULL (Constraints: `CREATED`, `OPENED`, `EDITED`, `REFERENCED`, `TAB_FOCUS`)
* `interaction_duration`: INTEGER DEFAULT 0 (Focus seconds)
* `captured_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `browser_sessions`
Captures raw tab navigation history from the browser connector.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE SET NULL
* `url`: TEXT NOT NULL
* `page_title`: TEXT NOT NULL
* `domain`: TEXT NOT NULL
* `visit_started_at`: TEXT NOT NULL
* `visit_ended_at`: TEXT
* `active_seconds`: INTEGER NOT NULL DEFAULT 0
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `search_queries`
Stores exact search text captured from engine inputs.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `browser_session_id`: INTEGER REFERENCES `browser_sessions(id)` ON DELETE CASCADE
* `query_text`: TEXT NOT NULL
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

### 2.2 Task, Commitment, & Planning Tables

#### Table: `commitments`
Parsed explicit obligations identified by the CDE.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE CASCADE
* `title`: TEXT NOT NULL
* `commitment_type`: TEXT NOT NULL (Constraints: `ASSIGNMENT`, `DELIVERABLE`, `MEETING`, `OBLIGATION`)
* `deadline_date`: TEXT (ISO-8601 string)
* `confidence_score`: REAL NOT NULL DEFAULT 1.0
* `source_node_id`: INTEGER REFERENCES `context_nodes(id)` ON DELETE SET NULL
* `status`: TEXT NOT NULL DEFAULT 'OPEN' (Constraints: `OPEN`, `COMPLETED`, `ABANDONED`)
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `project_deadlines`
Tracks target system milestones.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE CASCADE
* `deadline_label`: TEXT NOT NULL
* `target_date`: TEXT NOT NULL (ISO-8601 string)
* `importance_tier`: TEXT NOT NULL DEFAULT 'MEDIUM' (Constraints: `LOW`, `MEDIUM`, `HIGH`, `CRITICAL`)
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `project_actions`
Individual task execution records.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE CASCADE
* `action_text`: TEXT NOT NULL
* `estimated_effort_hours`: REAL NOT NULL DEFAULT 1.0
* `status`: TEXT NOT NULL DEFAULT 'PENDING' (Constraints: `PENDING`, `COMPLETED`, `DEPRECATED`)
* `priority_score`: REAL NOT NULL DEFAULT 0.0
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `project_checkpoints`
The human-submitted chronological checkpoints.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE CASCADE
* `accomplished_text`: TEXT NOT NULL
* `blocked_text`: TEXT
* `next_steps_text`: TEXT
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `recovery_plans`
Stores serialized lists of catch-up schedule actions.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `commitment_id`: INTEGER REFERENCES `commitments(id)` ON DELETE CASCADE
* `plan_payload_json`: TEXT NOT NULL (Stores the daily timeline checklist array)
* `generated_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `autonomous_research_briefs`
Caches the results generated during background research loops.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE CASCADE
* `brief_payload_json`: TEXT NOT NULL (Stores the array of matched bibliography nodes, repos, and summaries)
* `generated_at`: TEXT NOT NULL DEFAULT (datetime('now'))

### 2.3 Graph, Semantic, and Reconstruction Tables

#### Table: `graph_edges`
The directed adjacency table mapping the local semantic research graph.
* `source_node_id`: INTEGER REFERENCES `context_nodes(id)` ON DELETE CASCADE
* `target_node_id`: INTEGER REFERENCES `context_nodes(id)` ON DELETE CASCADE
* `edge_type`: TEXT NOT NULL (Constraints: `REFERENCES`, `GENERATED_FROM`, `SUPPORTS`, `BLOCKS`, `RELATED_TO`, `INVESTIGATES`, `DERIVED_FROM`)
* `weight`: REAL NOT NULL DEFAULT 1.0
* `created_at`: TEXT NOT NULL DEFAULT (datetime('now'))
* **Primary Key Constraint**: PRIMARY KEY (`source_node_id`, `target_node_id`, `edge_type`)

#### Table: `workspace_connector_registry`
Tracks connected IDE extension configurations.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `connector_id`: TEXT UNIQUE NOT NULL (Constraints: `VSCODE`, `CURSOR`, `WINDSURF`)
* `status`: TEXT NOT NULL DEFAULT 'ACTIVE'
* `last_pulsed_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `workspace_snapshots`
Caches raw editor window layouts for context reconstruction.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE CASCADE
* `active_file_path`: TEXT NOT NULL
* `cursor_line`: INTEGER NOT NULL DEFAULT 1
* `cursor_column`: INTEGER NOT NULL DEFAULT 1
* `open_tabs_json`: TEXT NOT NULL (Stores the serialized file paths array)
* `captured_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `project_snapshots`
Caches generated summary narratives.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `project_id`: INTEGER REFERENCES `projects(id)` ON DELETE CASCADE
* `snapshot_summary`: TEXT NOT NULL
* `workspace_state_json`: TEXT NOT NULL
* `generated_at`: TEXT NOT NULL DEFAULT (datetime('now'))

#### Table: `continuation_claims`
Factual assertions processed from snapshots.
* `id`: INTEGER PRIMARY KEY AUTOINCREMENT
* `snapshot_id`: INTEGER REFERENCES `project_snapshots(id)` ON DELETE CASCADE
* `claim_text`: TEXT NOT NULL
* `confidence_score`: REAL NOT NULL

#### Table: `continuation_claim_sources`
Direct mapping table of claims to source nodes.
* `claim_id`: INTEGER REFERENCES `continuation_claims(id)` ON DELETE CASCADE
* `node_id`: INTEGER REFERENCES `context_nodes(id)` ON DELETE CASCADE
* **Primary Key Constraint**: PRIMARY KEY (`claim_id`, `node_id`)

#### Table: `context_embeddings` (Virtual sqlite-vec Table)
* `node_id`: INTEGER PRIMARY KEY (Points directly to `context_nodes(id)`)
* `embedding`: FLOAT[384] (MiniLM-L6 vector space)

## 3. Performance Indexes

```sql
CREATE INDEX IF NOT EXISTS idx_ce_node ON context_events(node_id);
CREATE INDEX IF NOT EXISTS idx_cn_proj ON context_nodes(project_id);
CREATE INDEX IF NOT EXISTS idx_pa_proj ON project_actions(project_id, status);
CREATE INDEX IF NOT EXISTS idx_ws_proj ON workspace_snapshots(project_id);
CREATE INDEX IF NOT EXISTS idx_cmt_proj ON commitments(project_id, status);
CREATE INDEX IF NOT EXISTS idx_brw_proj ON browser_sessions(project_id);
CREATE INDEX IF NOT EXISTS idx_srch_sess ON search_queries(browser_session_id);
```

## 4. Retention Policy (Local Vacuum and Maintenance)
To guarantee the local SQLite database does not bloat, the Rust Daemon triggers a cleanup task on boot:
* **Event Pruning**: Deletes rows inside `context_events` older than 30 days, keeping the `context_nodes` references.
* **Browser Session Pruning**: Keeps rows inside `browser_sessions` and `search_queries` for up to 14 days, deleting entries with active focus durations under 5.0 seconds.
* **Database Maintenance**: Triggers a `VACUUM` when database size on disk exceeds 250MB.
