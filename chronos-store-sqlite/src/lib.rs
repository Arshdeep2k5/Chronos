//! # Chronos SQLite Event Store
//!
//! A production-ready, durable implementation of the `EventStore` trait using SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use chronos_store::{EventStore, StoreError};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A durable EventStore backed by a local SQLite database file.
pub struct SQLiteEventStore {
    pub conn: Arc<Mutex<Connection>>,
}

impl SQLiteEventStore {
    /// Creates or opens an SQLite database at the specified path and initializes tables.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let conn = Connection::open(path)
            .map_err(|_| StoreError::Unavailable)?;
            
        // Enable WAL mode and configure parameters
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;"
        ).map_err(|e| StoreError::AppendError(format!("Failed to set PRAGMAs: {}", e)))?;

        // Initialize the events table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chronos_events (
                id             TEXT PRIMARY KEY,
                timestamp      TEXT NOT NULL,
                schema_version TEXT NOT NULL,
                event_type     TEXT NOT NULL,
                source         TEXT NOT NULL,
                payload        TEXT NOT NULL
            );",
            [],
        ).map_err(|e| StoreError::AppendError(format!("Failed to create table: {}", e)))?;

        // Initialize the alerts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chronos_alerts (
                alert_id       TEXT PRIMARY KEY,
                event_id       TEXT NOT NULL,
                timestamp      TEXT NOT NULL,
                severity       TEXT NOT NULL,
                component      TEXT NOT NULL,
                event_type     TEXT NOT NULL,
                message        TEXT NOT NULL,
                metadata       TEXT NOT NULL,
                FOREIGN KEY(event_id) REFERENCES chronos_events(id) ON DELETE CASCADE
            );",
            [],
        ).map_err(|e| StoreError::AppendError(format!("Failed to create alerts table: {}", e)))?;

        // Create performance indices
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON chronos_events(timestamp);
             CREATE INDEX IF NOT EXISTS idx_events_event_type ON chronos_events(event_type);
             CREATE INDEX IF NOT EXISTS idx_alerts_severity ON chronos_alerts(severity);
             CREATE INDEX IF NOT EXISTS idx_alerts_timestamp ON chronos_alerts(timestamp);"
        ).map_err(|e| StoreError::AppendError(format!("Failed to create indices: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn index_alert_if_needed(conn: &rusqlite::Connection, event: &ChronosEvent) -> Result<(), StoreError> {
        let severity = match event.event_type.as_str() {
            "TickPerformanceWarning" => Some("WARN"),
            "UiTelemetryAckReceived" => Some("INFO"),
            "ActionFailed" | "ExecutionFailed" => Some("CRITICAL"),
            "StreamHealthDegraded" | "DroppedFrameDetected" => Some("WARN"),
            _ => None,
        };

        if let Some(sev) = severity {
            let alert_id = uuid::Uuid::new_v4().to_string();
            let msg = match event.event_type.as_str() {
                "TickPerformanceWarning" => event.payload.get("warning").and_then(|v| v.as_str()).unwrap_or("Slow tick processing detected."),
                "UiTelemetryAckReceived" => "UI tick frame render confirmation received.",
                "ActionFailed" | "ExecutionFailed" => event.payload.get("error").and_then(|v| v.as_str()).unwrap_or("Execution phase failed."),
                _ => "System warning or health alert.",
            };

            let payload_str = serde_json::to_string(&event.payload).unwrap_or_default();

            conn.execute(
                "INSERT INTO chronos_alerts (alert_id, event_id, timestamp, severity, component, event_type, message, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    alert_id,
                    event.id,
                    event.timestamp.to_rfc3339(),
                    sev,
                    event.source,
                    event.event_type,
                    msg,
                    payload_str,
                ],
            ).map_err(|e| StoreError::AppendError(format!("Alert index failed: {}", e)))?;
        }

        Ok(())
    }
}

#[async_trait]
impl EventStore for SQLiteEventStore {
    async fn append(&self, event: ChronosEvent) -> Result<(), StoreError> {
        let lock = self.conn.lock().await;
        let payload_str = serde_json::to_string(&event.payload)
            .map_err(|e| StoreError::AppendError(format!("Serialization error: {}", e)))?;
            
        lock.execute(
            "INSERT INTO chronos_events (id, timestamp, schema_version, event_type, source, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                event.timestamp.to_rfc3339(),
                event.schema_version,
                event.event_type,
                event.source,
                payload_str,
            ],
        ).map_err(|e| StoreError::AppendError(format!("Database insert failed: {}", e)))?;
        
        let _ = Self::index_alert_if_needed(&lock, &event);
        
        Ok(())
    }

    async fn append_batch(&self, events: Vec<ChronosEvent>) -> Result<(), StoreError> {
        let mut lock = self.conn.lock().await;
        
        // Wrap batch insertion in an explicit database transaction
        let tx = lock.transaction()
            .map_err(|e| StoreError::AppendError(format!("Failed to begin transaction: {}", e)))?;
        
        {
            let mut stmt = tx.prepare(
                "INSERT INTO chronos_events (id, timestamp, schema_version, event_type, source, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            ).map_err(|e| StoreError::AppendError(format!("Preparation failed: {}", e)))?;
            
            for event in events {
                let payload_str = serde_json::to_string(&event.payload)
                    .map_err(|e| StoreError::AppendError(format!("Serialization error: {}", e)))?;
                stmt.execute(params![
                    event.id,
                    event.timestamp.to_rfc3339(),
                    event.schema_version,
                    event.event_type,
                    event.source,
                    payload_str,
                ]).map_err(|e| StoreError::AppendError(format!("Batch insert failed: {}", e)))?;
                
                let _ = Self::index_alert_if_needed(&tx, &event);
            }
        }
        
        tx.commit()
            .map_err(|e| StoreError::AppendError(format!("Transaction commit failed: {}", e)))?;
            
        Ok(())
    }

    async fn get(&self, event_id: &str) -> Result<Option<ChronosEvent>, StoreError> {
        let lock = self.conn.lock().await;
        let mut stmt = lock.prepare(
            "SELECT id, timestamp, schema_version, event_type, source, payload
             FROM chronos_events WHERE id = ?1"
        ).map_err(|e| StoreError::RetrieveError(format!("Query prep failed: {}", e)))?;
        
        let mut rows = stmt.query(params![event_id])
            .map_err(|e| StoreError::RetrieveError(format!("Query execution failed: {}", e)))?;
            
        if let Some(row) = rows.next().map_err(|e| StoreError::RetrieveError(e.to_string()))? {
            let timestamp_str: String = row.get(1).map_err(|e| StoreError::RetrieveError(e.to_string()))?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| StoreError::RetrieveError(format!("Timestamp parse error: {}", e)))?
                .with_timezone(&Utc);
                
            let payload_str: String = row.get(5).map_err(|e| StoreError::RetrieveError(e.to_string()))?;
            let payload = serde_json::from_str(&payload_str)
                .map_err(|e| StoreError::RetrieveError(format!("Payload parse error: {}", e)))?;
                
            Ok(Some(ChronosEvent {
                id: row.get(0).map_err(|e| StoreError::RetrieveError(e.to_string()))?,
                timestamp,
                schema_version: row.get(2).map_err(|e| StoreError::RetrieveError(e.to_string()))?,
                event_type: row.get(3).map_err(|e| StoreError::RetrieveError(e.to_string()))?,
                source: row.get(4).map_err(|e| StoreError::RetrieveError(e.to_string()))?,
                payload,
            }))
        } else {
            Ok(None)
        }
    }

    async fn stream(&self) -> Result<Vec<ChronosEvent>, StoreError> {
        let lock = self.conn.lock().await;
        let mut stmt = lock.prepare(
            "SELECT id, timestamp, schema_version, event_type, source, payload
             FROM chronos_events ORDER BY timestamp ASC"
        ).map_err(|e| StoreError::RetrieveError(format!("Query prep failed: {}", e)))?;
        
        let mapped = stmt.query_map([], |row| {
            let timestamp_str: String = row.get(1)?;
            let payload_str: String = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                timestamp_str,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                payload_str,
            ))
        }).map_err(|e| StoreError::RetrieveError(format!("Query failed: {}", e)))?;
        
        let mut events = Vec::new();
        for r in mapped {
            let (id, t_str, ver, ev_type, src, pay_str) = r.map_err(|e| StoreError::RetrieveError(e.to_string()))?;
            let timestamp = DateTime::parse_from_rfc3339(&t_str)
                .map_err(|e| StoreError::RetrieveError(format!("Timestamp parse error: {}", e)))?
                .with_timezone(&Utc);
            let payload = serde_json::from_str(&pay_str)
                .map_err(|e| StoreError::RetrieveError(format!("Payload parse error: {}", e)))?;
                
            events.push(ChronosEvent {
                id,
                timestamp,
                schema_version: ver,
                event_type: ev_type,
                source: src,
                payload,
            });
        }
        
        Ok(events)
    }

    async fn replay(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ChronosEvent>, StoreError> {
        let lock = self.conn.lock().await;
        let mut stmt = lock.prepare(
            "SELECT id, timestamp, schema_version, event_type, source, payload
             FROM chronos_events
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp ASC"
        ).map_err(|e| StoreError::RetrieveError(format!("Query prep failed: {}", e)))?;
        
        let mapped = stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
            let timestamp_str: String = row.get(1)?;
            let payload_str: String = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                timestamp_str,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                payload_str,
            ))
        }).map_err(|e| StoreError::RetrieveError(format!("Query failed: {}", e)))?;
        
        let mut events = Vec::new();
        for r in mapped {
            let (id, t_str, ver, ev_type, src, pay_str) = r.map_err(|e| StoreError::RetrieveError(e.to_string()))?;
            let timestamp = DateTime::parse_from_rfc3339(&t_str)
                .map_err(|e| StoreError::RetrieveError(format!("Timestamp parse error: {}", e)))?
                .with_timezone(&Utc);
            let payload = serde_json::from_str(&pay_str)
                .map_err(|e| StoreError::RetrieveError(format!("Payload parse error: {}", e)))?;
                
            events.push(ChronosEvent {
                id,
                timestamp,
                schema_version: ver,
                event_type: ev_type,
                source: src,
                payload,
            });
        }
        
        Ok(events)
    }

    async fn latest(&self, n: usize) -> Result<Vec<ChronosEvent>, StoreError> {
        let lock = self.conn.lock().await;
        let mut stmt = lock.prepare(
            "SELECT id, timestamp, schema_version, event_type, source, payload
             FROM chronos_events
             ORDER BY timestamp DESC
             LIMIT ?1"
        ).map_err(|e| StoreError::RetrieveError(format!("Query prep failed: {}", e)))?;
        
        let mapped = stmt.query_map(params![n], |row| {
            let timestamp_str: String = row.get(1)?;
            let payload_str: String = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                timestamp_str,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                payload_str,
            ))
        }).map_err(|e| StoreError::RetrieveError(format!("Query failed: {}", e)))?;
        
        let mut events = Vec::new();
        for r in mapped {
            let (id, t_str, ver, ev_type, src, pay_str) = r.map_err(|e| StoreError::RetrieveError(e.to_string()))?;
            let timestamp = DateTime::parse_from_rfc3339(&t_str)
                .map_err(|e| StoreError::RetrieveError(format!("Timestamp parse error: {}", e)))?
                .with_timezone(&Utc);
            let payload = serde_json::from_str(&pay_str)
                .map_err(|e| StoreError::RetrieveError(format!("Payload parse error: {}", e)))?;
                
            events.push(ChronosEvent {
                id,
                timestamp,
                schema_version: ver,
                event_type: ev_type,
                source: src,
                payload,
            });
        }
        
        // Reverse so that the returned collection preserves chronological order
        events.reverse();
        Ok(events)
    }

    async fn count(&self) -> Result<usize, StoreError> {
        let lock = self.conn.lock().await;
        let count: usize = lock.query_row(
            "SELECT COUNT(*) FROM chronos_events",
            [],
            |row| row.get(0),
        ).map_err(|e| StoreError::RetrieveError(e.to_string()))?;
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_sqlite_single_append_and_get() {
        let temp = NamedTempFile::new().unwrap();
        let store = SQLiteEventStore::new(temp.path()).unwrap();
        
        let event = ChronosEvent::new("FileOpen", "VSCode", json!({"path": "/src/lib.rs"}));
        let id = event.id.clone();
        
        store.append(event).await.unwrap();
        
        let retrieved = store.get(&id).await.unwrap().expect("Event should exist");
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.event_type, "FileOpen");
        assert_eq!(retrieved.source, "VSCode");
        assert_eq!(retrieved.payload["path"], "/src/lib.rs");
    }

    #[tokio::test]
    async fn test_sqlite_batch_append_and_count() {
        let temp = NamedTempFile::new().unwrap();
        let store = SQLiteEventStore::new(temp.path()).unwrap();
        
        let events = vec![
            ChronosEvent::new("Ev1", "Src", json!({})),
            ChronosEvent::new("Ev2", "Src", json!({})),
            ChronosEvent::new("Ev3", "Src", json!({})),
        ];
        
        store.append_batch(events).await.unwrap();
        
        let count = store.count().await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_sqlite_replay_ordering() {
        let temp = NamedTempFile::new().unwrap();
        let store = SQLiteEventStore::new(temp.path()).unwrap();
        
        let t1 = Utc::now();
        let mut e1 = ChronosEvent::new("First", "Src", json!({}));
        e1.timestamp = t1;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        let t2 = Utc::now();
        let mut e2 = ChronosEvent::new("Second", "Src", json!({}));
        e2.timestamp = t2;
        
        store.append(e2).await.unwrap(); // Append Second first to test sorting
        store.append(e1).await.unwrap();
        
        let history = store.stream().await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].event_type, "First");
        assert_eq!(history[1].event_type, "Second");
    }

    #[tokio::test]
    async fn test_sqlite_latest() {
        let temp = NamedTempFile::new().unwrap();
        let store = SQLiteEventStore::new(temp.path()).unwrap();
        
        for i in 0..10 {
            store.append(ChronosEvent::new(format!("Ev{}", i), "Src", json!({}))).await.unwrap();
        }
        
        let latest = store.latest(3).await.unwrap();
        assert_eq!(latest.len(), 3);
        assert_eq!(latest[0].event_type, "Ev7");
        assert_eq!(latest[1].event_type, "Ev8");
        assert_eq!(latest[2].event_type, "Ev9");
    }

    #[tokio::test]
    async fn test_sqlite_persistence_across_restarts() {
        let temp = NamedTempFile::new().unwrap();
        let db_path = temp.path().to_path_buf();
        
        let event_id = {
            let store = SQLiteEventStore::new(&db_path).unwrap();
            let event = ChronosEvent::new("PersistMe", "Test", json!({}));
            let id = event.id.clone();
            store.append(event).await.unwrap();
            id
        }; // store dropped here
        
        // Reopen database
        let store2 = SQLiteEventStore::new(&db_path).unwrap();
        let retrieved = store2.get(&event_id).await.unwrap().expect("Should exist after reload");
        assert_eq!(retrieved.id, event_id);
    }
}
