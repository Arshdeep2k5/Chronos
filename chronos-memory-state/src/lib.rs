//! # Chronos State Projector
//!
//! Materializes the global `ChronosState` world model projection from underlying
//! entity graphs, event logs, and active session streams.

use chrono::{DateTime, Utc};
use chronos_core::ChronosState;
use chronos_memory_entity_resolution::{EntityGraph, EntityType};
use chronos_memory_sessions::SessionProjection;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Error types for the State Projector.
#[derive(Debug, thiserror::Error)]
pub enum ProjectorError {
    #[error("Replay execution error: {0}")]
    ReplayError(String),
}

/// The metadata payload representing the materialized projection within ChronosState.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectedStatePayload {
    pub freshness_timestamp: DateTime<Utc>,
    pub aggregated_confidence: f64,
    pub active_session_id: Option<String>,
    pub active_entity_ids: Vec<String>,
    pub dormant_entity_ids: Vec<String>,
    pub archived_entity_ids: Vec<String>,
    pub incomplete_entity_ids: Vec<String>,
    pub provenance_event_ids: Vec<String>,
}

pub struct StateProjector;

impl StateProjector {
    /// Projects the current state from the entity graph and session projection.
    pub fn project(
        graph: &EntityGraph,
        sessions: &SessionProjection,
    ) -> ChronosState {
        let active_session = sessions.latest().filter(|s| !s.is_closed);
        let active_session_id = active_session.map(|s| s.session_id.clone());

        // Collect active entities directly referenced in the active session
        let active_entities: HashSet<String> = active_session
            .map(|s| s.entity_ids.clone())
            .unwrap_or_default();

        let mut dormant_entities = HashSet::new();
        let mut archived_entities = HashSet::new();
        let mut incomplete_entities = HashSet::new();

        // Track all provenance event IDs
        let mut provenance_event_ids = HashSet::new();
        if let Some(sess) = active_session {
            for id in &sess.source_event_ids {
                provenance_event_ids.insert(id.clone());
            }
        }

        // Evaluate all entities in the graph
        for entity in graph.entities().values() {
            // Provenance check
            for evt_id in &entity.provenance {
                provenance_event_ids.insert(evt_id.clone());
            }

            if active_entities.contains(&entity.id) {
                continue;
            }

            // Incomplete check: a File entity with no parent Repository reference in the relationships
            if entity.entity_type == EntityType::File {
                let has_parent = graph.relationships().iter().any(|r| {
                    r.target_id == entity.id && r.relation_type == "owns_file"
                });
                if !has_parent {
                    incomplete_entities.insert(entity.id.clone());
                    continue;
                }
            }

            // Dormant check: if entity is in a closed session that is less than 3600 seconds old
            let mut is_dormant = false;
            for sess in sessions.sessions().values() {
                if sess.is_closed && sess.entity_ids.contains(&entity.id) {
                    let age = (Utc::now() - sess.end_timestamp).num_seconds();
                    if age < 3600 {
                        is_dormant = true;
                    }
                }
            }

            if is_dormant {
                dormant_entities.insert(entity.id.clone());
            } else {
                archived_entities.insert(entity.id.clone());
            }
        }

        // Calculate aggregated confidence (rolling average of entity confidence indexes)
        let total_entities = graph.entities().len();
        let aggregated_confidence = if total_entities > 0 {
            let sum: f64 = graph.entities().values().map(|e| e.confidence).sum();
            sum / total_entities as f64
        } else {
            1.0
        };

        // Construct payload details
        let payload_struct = ProjectedStatePayload {
            freshness_timestamp: Utc::now(),
            aggregated_confidence,
            active_session_id: active_session_id.clone(),
            active_entity_ids: active_entities.into_iter().collect(),
            dormant_entity_ids: dormant_entities.into_iter().collect(),
            archived_entity_ids: archived_entities.into_iter().collect(),
            incomplete_entity_ids: incomplete_entities.into_iter().collect(),
            provenance_event_ids: provenance_event_ids.into_iter().collect(),
        };

        let payload_json = serde_json::to_value(&payload_struct).unwrap_or(serde_json::Value::Null);

        // Map active intents to active session IDs or active projects
        let mut active_intents = Vec::new();
        if let Some(ref id) = active_session_id {
            active_intents.push(id.clone());
        }

        ChronosState::new(active_intents, Vec::new(), payload_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_core::ChronosEvent;
    use chronos_memory_entity_resolution::EntityResolver;
    use chronos_memory_sessions::SessionEngine;
    use serde_json::json;

    #[test]
    fn test_active_and_incomplete_classification() {
        let mut resolver = EntityResolver::new();
        
        // Discovered Repo (creates Repository node)
        let disc_event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos" }),
        );
        resolver.process_event(&disc_event).unwrap();

        // Create a File event that is NOT linked to Repository (meaning it is Incomplete)
        // For file resolution, ResolveFileRule runs on GitCommitCreated.
        // Let's create a commit that references "src/lib.rs"
        let commit_event = ChronosEvent::new(
            "GitCommitCreated",
            "GitAdapter",
            json!({
                "repository_path": "/workspace/chronos",
                "source_payload": {
                    "message": "commit",
                    "files": ["src/lib.rs"]
                }
            }),
        );
        resolver.process_event(&commit_event).unwrap();

        let graph = resolver.graph();
        
        // Setup session engine
        let mut session_engine = SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc_event).unwrap();
        session_engine.process_event(&commit_event).unwrap();

        let state = StateProjector::project(graph, session_engine.projection());
        
        // Deserialize payload
        let payload: ProjectedStatePayload = serde_json::from_value(state.payload.clone()).unwrap();

        // The active session should contain the repository and file
        assert!(payload.active_session_id.is_some());
        assert!(!payload.active_entity_ids.is_empty());
        assert_eq!(payload.incomplete_entity_ids.len(), 0); // linked cleanly to Repository via rules
    }

    #[test]
    fn test_replay_determinism() {
        let mut resolver = EntityResolver::new();
        let disc_event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos" }),
        );
        resolver.process_event(&disc_event).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc_event).unwrap();

        let state1 = StateProjector::project(&graph, session_engine.projection());

        // Re-projecting must yield the exact same structure (excluding local Utc::now() timestamp differences in subfields, 
        // or we assert fields match exactly)
        let state2 = StateProjector::project(&graph, session_engine.projection());
        
        let p1: ProjectedStatePayload = serde_json::from_value(state1.payload).unwrap();
        let p2: ProjectedStatePayload = serde_json::from_value(state2.payload).unwrap();

        assert_eq!(p1.active_session_id, p2.active_session_id);
        assert_eq!(p1.active_entity_ids, p2.active_entity_ids);
        assert_eq!(p1.dormant_entity_ids, p2.dormant_entity_ids);
    }
}
