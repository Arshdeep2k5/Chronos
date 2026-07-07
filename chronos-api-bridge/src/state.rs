use std::sync::Arc;

use chronos_bus::MemoryEventBus;
use chronos_memory_entity_resolution::EntityResolver;
use chronos_memory_sessions::SessionEngine;
use chronos_registry::ServiceRegistry;
use chronos_store_sqlite::SQLiteEventStore;
use chronos_reasoning_commitments::CommitmentState;
use chronos_event_orchestrator::EventOrchestrator;
use tokio::sync::RwLock;

/// Shared live state for the API bridge.
/// Wrapped in Arc for cheap cloning across Axum handlers.
#[derive(Clone)]
pub struct BridgeState {
    /// Durable SQLite event store — source of truth for all persisted events.
    pub store: Arc<SQLiteEventStore>,
    /// Live service registry — advertises active service capabilities.
    pub registry: Arc<ServiceRegistry>,
    /// In-memory entity resolver — rebuilt from event replay on startup.
    pub resolver: Arc<RwLock<EntityResolver>>,
    /// In-memory session engine — rebuilt from event replay on startup.
    pub session_engine: Arc<RwLock<Option<SessionEngine>>>,
    /// Live commitment state — rebuilt from event replay on startup.
    pub commitments: Arc<RwLock<CommitmentState>>,
    /// Live event orchestrator
    pub orchestrator: Arc<RwLock<EventOrchestrator>>,
    /// Cognitive Bus — used by the ingest endpoint to publish perception events.
    pub bus: Arc<MemoryEventBus>,
}

impl BridgeState {
    pub fn new(
        store: Arc<SQLiteEventStore>,
        registry: Arc<ServiceRegistry>,
        resolver: Arc<RwLock<EntityResolver>>,
        session_engine: Arc<RwLock<Option<SessionEngine>>>,
        commitments: Arc<RwLock<CommitmentState>>,
        orchestrator: Arc<RwLock<EventOrchestrator>>,
        bus: Arc<MemoryEventBus>,
    ) -> Self {
        Self {
            store,
            registry,
            resolver,
            session_engine,
            commitments,
            orchestrator,
            bus,
        }
    }
}
