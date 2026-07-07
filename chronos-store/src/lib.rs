//! # Chronos Event Store
//!
//! The Event Store is the canonical persistence abstraction responsible for recording and replaying `ChronosEvent`s.
//! It is NOT tied directly to SQLite or any specific database implementation.
//! 
//! Only `ChronosEvent` is persisted here. No graph nodes, no reasoning state, no AI context.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Errors that can occur within an EventStore.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Failed to append event: {0}")]
    AppendError(String),
    #[error("Failed to retrieve event: {0}")]
    RetrieveError(String),
    #[error("Event store unavailable or disconnected")]
    Unavailable,
}

/// The core interface for durable event persistence.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Appends a single event to the store.
    async fn append(&self, event: ChronosEvent) -> Result<(), StoreError>;

    /// Appends a batch of events to the store in an atomic-like fashion (implementation dependent).
    async fn append_batch(&self, events: Vec<ChronosEvent>) -> Result<(), StoreError>;

    /// Retrieves a single event by its unique ID.
    async fn get(&self, event_id: &str) -> Result<Option<ChronosEvent>, StoreError>;

    /// Retrieves all events from the store (conceptually streaming them).
    /// In a production environment with millions of events, this would return an AsyncIterator/Stream.
    /// For the abstraction, returning a Vec represents the fully collected stream.
    async fn stream(&self) -> Result<Vec<ChronosEvent>, StoreError>;

    /// Replays events that occurred between the `start` and `end` timestamps.
    async fn replay(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ChronosEvent>, StoreError>;

    /// Retrieves the most recent `n` events.
    async fn latest(&self, n: usize) -> Result<Vec<ChronosEvent>, StoreError>;

    /// Returns the total number of events in the store.
    async fn count(&self) -> Result<usize, StoreError>;
}

/// An in-memory implementation of the EventStore for testing and fast local execution.
pub struct MemoryEventStore {
    events: Arc<RwLock<Vec<ChronosEvent>>>,
}

impl MemoryEventStore {
    /// Creates a new, empty MemoryEventStore.
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for MemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStore for MemoryEventStore {
    async fn append(&self, event: ChronosEvent) -> Result<(), StoreError> {
        let mut lock = self.events.write().await;
        lock.push(event);
        Ok(())
    }

    async fn append_batch(&self, events: Vec<ChronosEvent>) -> Result<(), StoreError> {
        let mut lock = self.events.write().await;
        for event in events {
            lock.push(event);
        }
        Ok(())
    }

    async fn get(&self, event_id: &str) -> Result<Option<ChronosEvent>, StoreError> {
        let lock = self.events.read().await;
        Ok(lock.iter().find(|e| e.id == event_id).cloned())
    }

    async fn stream(&self) -> Result<Vec<ChronosEvent>, StoreError> {
        let lock = self.events.read().await;
        Ok(lock.clone())
    }

    async fn replay(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ChronosEvent>, StoreError> {
        let lock = self.events.read().await;
        let filtered: Vec<ChronosEvent> = lock
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn latest(&self, n: usize) -> Result<Vec<ChronosEvent>, StoreError> {
        let lock = self.events.read().await;
        let len = lock.len();
        let start_idx = if len > n { len - n } else { 0 };
        
        let slice = &lock[start_idx..];
        Ok(slice.to_vec())
    }

    async fn count(&self) -> Result<usize, StoreError> {
        let lock = self.events.read().await;
        Ok(lock.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn test_append_and_get() {
        let store = MemoryEventStore::new();
        let event = ChronosEvent::new("TestEvent", "Source", json!({}));
        let id = event.id.clone();
        
        store.append(event).await.unwrap();
        
        let retrieved = store.get(&id).await.unwrap().expect("Event not found");
        assert_eq!(retrieved.id, id);
    }

    #[tokio::test]
    async fn test_append_batch_and_count() {
        let store = MemoryEventStore::new();
        let events = vec![
            ChronosEvent::new("E1", "S1", json!({})),
            ChronosEvent::new("E2", "S2", json!({})),
            ChronosEvent::new("E3", "S3", json!({})),
        ];
        
        store.append_batch(events).await.unwrap();
        
        let count = store.count().await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_latest() {
        let store = MemoryEventStore::new();
        for i in 0..5 {
            store.append(ChronosEvent::new(format!("E{}", i), "S", json!({}))).await.unwrap();
        }
        
        let latest_events = store.latest(2).await.unwrap();
        assert_eq!(latest_events.len(), 2);
        assert_eq!(latest_events[0].event_type, "E3");
        assert_eq!(latest_events[1].event_type, "E4");
    }

    #[tokio::test]
    async fn test_replay() {
        let store = MemoryEventStore::new();
        
        let start_time = Utc::now();
        
        // Wait a tiny bit just to ensure timestamp difference, though not strictly necessary 
        // if we just manipulate dates, but for an actual test we can just let it run.
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        store.append(ChronosEvent::new("E1", "S", json!({}))).await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mid_time = Utc::now();
        tokio::time::sleep(Duration::from_millis(10)).await;

        store.append(ChronosEvent::new("E2", "S", json!({}))).await.unwrap();

        let end_time = Utc::now();

        // Replay from start to mid (should only contain E1)
        let replayed = store.replay(start_time, mid_time).await.unwrap();
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].event_type, "E1");

        // Replay from mid to end (should only contain E2)
        let replayed_late = store.replay(mid_time, end_time).await.unwrap();
        assert_eq!(replayed_late.len(), 1);
        assert_eq!(replayed_late[0].event_type, "E2");
    }
}
