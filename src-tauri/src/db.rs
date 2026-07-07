use rusqlite::{Connection, Result};
use std::path::Path;
use std::fs;

pub fn init_db(db_path: &Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).unwrap_or_default();
    }
    
    let conn = Connection::open(db_path)?;
    
    // Enable PRAGMAs
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 300000000;"
    )?;
    
    // Create tables
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS dead_letter_queue (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            source_uri          TEXT NOT NULL,
            payload_hash        TEXT NOT NULL UNIQUE,
            worker_type         TEXT NOT NULL,
            failure_reason      TEXT NOT NULL,
            retry_count         INTEGER NOT NULL DEFAULT 0,
            failed_at           TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS projects (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_name        TEXT UNIQUE NOT NULL,
            status              TEXT NOT NULL DEFAULT 'ACTIVE',
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS project_state (
            project_id          INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
            current_summary     TEXT NOT NULL,
            current_entry_point TEXT,
            next_action         TEXT,
            confidence_score    REAL NOT NULL DEFAULT 1.0,
            updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS context_nodes (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            entity_key          TEXT UNIQUE NOT NULL,
            entity_type         TEXT NOT NULL,
            display_name        TEXT NOT NULL,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS context_events (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id             INTEGER REFERENCES context_nodes(id) ON DELETE CASCADE,
            event_type          TEXT NOT NULL,
            interaction_duration INTEGER DEFAULT 0,
            captured_at         TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS browser_sessions (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            url                 TEXT NOT NULL,
            page_title          TEXT NOT NULL,
            domain              TEXT NOT NULL,
            visit_started_at    TEXT NOT NULL,
            visit_ended_at      TEXT,
            active_seconds      INTEGER NOT NULL DEFAULT 0,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS search_queries (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            browser_session_id  INTEGER REFERENCES browser_sessions(id) ON DELETE CASCADE,
            query_text          TEXT NOT NULL,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS commitments (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE CASCADE,
            title               TEXT NOT NULL,
            commitment_type     TEXT NOT NULL,
            deadline_date       TEXT,
            confidence_score    REAL NOT NULL DEFAULT 1.0,
            source_node_id      INTEGER REFERENCES context_nodes(id) ON DELETE SET NULL,
            status              TEXT NOT NULL DEFAULT 'OPEN',
            estimated_effort_hours REAL NOT NULL DEFAULT 2.5,
            risk_flagged        INTEGER NOT NULL DEFAULT 0,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS project_deadlines (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE CASCADE,
            deadline_label      TEXT NOT NULL,
            target_date         TEXT NOT NULL,
            importance_tier     TEXT NOT NULL DEFAULT 'MEDIUM',
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS project_actions (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE CASCADE,
            action_text         TEXT NOT NULL,
            estimated_effort_hours REAL NOT NULL DEFAULT 1.0,
            status              TEXT NOT NULL DEFAULT 'PENDING',
            priority_score      REAL NOT NULL DEFAULT 0.0,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS project_checkpoints (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE CASCADE,
            accomplished_text   TEXT NOT NULL,
            blocked_text        TEXT,
            next_steps_text     TEXT,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS recovery_plans (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            commitment_id       INTEGER REFERENCES commitments(id) ON DELETE CASCADE,
            plan_payload_json   TEXT NOT NULL,
            generated_at        TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS autonomous_research_briefs (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE CASCADE,
            brief_payload_json  TEXT NOT NULL,
            generated_at        TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS graph_edges (
            source_node_id      INTEGER REFERENCES context_nodes(id) ON DELETE CASCADE,
            target_node_id      INTEGER REFERENCES context_nodes(id) ON DELETE CASCADE,
            edge_type           TEXT NOT NULL,
            weight              REAL NOT NULL DEFAULT 1.0,
            created_at          TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (source_node_id, target_node_id, edge_type)
        );

        CREATE TABLE IF NOT EXISTS workspace_connector_registry (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            connector_id        TEXT UNIQUE NOT NULL,
            status              TEXT NOT NULL DEFAULT 'ACTIVE',
            last_pulsed_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS workspace_snapshots (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE CASCADE,
            active_file_path    TEXT NOT NULL,
            cursor_line         INTEGER NOT NULL DEFAULT 1,
            cursor_column       INTEGER NOT NULL DEFAULT 1,
            open_tabs_json      TEXT NOT NULL,
            captured_at         TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS project_snapshots (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER REFERENCES projects(id) ON DELETE CASCADE,
            snapshot_summary    TEXT NOT NULL,
            workspace_state_json TEXT NOT NULL,
            generated_at        TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS continuation_claims (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id         INTEGER REFERENCES project_snapshots(id) ON DELETE CASCADE,
            claim_text          TEXT NOT NULL,
            confidence_score    REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS continuation_claim_sources (
            claim_id            INTEGER REFERENCES continuation_claims(id) ON DELETE CASCADE,
            node_id             INTEGER REFERENCES context_nodes(id) ON DELETE CASCADE,
            PRIMARY KEY (claim_id, node_id)
        );"
    )?;

    // Handle context_embeddings virtual table if loadable, fallback otherwise.
    // We try to run virtual table create directly; if it fails, we fall back.
    if conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS context_embeddings USING vec0(
            node_id             INTEGER PRIMARY KEY,
            embedding           FLOAT[384] distance_metric=cosine
        );"
    ).is_err() {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS context_embeddings (
                node_id             INTEGER PRIMARY KEY REFERENCES context_nodes(id) ON DELETE CASCADE,
                embedding           TEXT NOT NULL
            );"
        )?;
    }

    // Indices
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_ce_node ON context_events(node_id);
         CREATE INDEX IF NOT EXISTS idx_cn_proj ON context_nodes(project_id);
         CREATE INDEX IF NOT EXISTS idx_pa_proj ON project_actions(project_id, status);
         CREATE INDEX IF NOT EXISTS idx_ws_proj ON workspace_snapshots(project_id);
         CREATE INDEX IF NOT EXISTS idx_cmt_proj ON commitments(project_id, status);
         CREATE INDEX IF NOT EXISTS idx_brw_proj ON browser_sessions(project_id);
         CREATE INDEX IF NOT EXISTS idx_srch_sess ON search_queries(browser_session_id);"
    )?;

    // Add new columns if they don't exist (ignore errors if they do)
    let _ = conn.execute("ALTER TABLE commitments ADD COLUMN estimated_effort_hours REAL NOT NULL DEFAULT 2.5", []);
    let _ = conn.execute("ALTER TABLE commitments ADD COLUMN risk_flagged INTEGER NOT NULL DEFAULT 0", []);

    seed_db(&conn)?;

    Ok(conn)
}

pub fn seed_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO projects (id, project_name, status) VALUES (1, 'Chronos Pilot Project', 'ACTIVE')",
        [],
    )?;
    Ok(())
}

pub fn init_telemetry_db(db_path: &Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).unwrap_or_default();
    }
    
    let conn = Connection::open(db_path)?;
    
    // Enable PRAGMAs
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 300000000;"
    )?;
    
    // Create ephemeral telemetry tables without foreign keys
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS browser_sessions (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER,
            url                 TEXT NOT NULL,
            page_title          TEXT NOT NULL,
            domain              TEXT NOT NULL,
            visit_started_at    TEXT NOT NULL,
            visit_ended_at      TEXT,
            active_seconds      INTEGER NOT NULL DEFAULT 0,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS search_queries (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            browser_session_id  INTEGER,
            query_text          TEXT NOT NULL,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS context_events (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id             INTEGER,
            event_type          TEXT NOT NULL,
            interaction_duration INTEGER DEFAULT 0,
            captured_at         TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS workspace_snapshots (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id          INTEGER,
            active_file_path    TEXT NOT NULL,
            cursor_line         INTEGER NOT NULL DEFAULT 1,
            cursor_column       INTEGER NOT NULL DEFAULT 1,
            open_tabs_json      TEXT NOT NULL,
            captured_at         TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )?;

    // Indices for fast reads
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_brw_created ON browser_sessions(created_at);
         CREATE INDEX IF NOT EXISTS idx_ws_created ON workspace_snapshots(captured_at);
         CREATE INDEX IF NOT EXISTS idx_ce_created ON context_events(captured_at);"
    )?;

    Ok(conn)
}
