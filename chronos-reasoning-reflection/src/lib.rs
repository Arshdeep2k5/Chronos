//! # Chronos Reflection Engine
//!
//! Evaluates active state, entity networks, and session limits to produce explainable
//! interpretations of reality (ChronosReflection).

use chrono::{DateTime, Utc};
use chronos_core::ChronosReflection;
use chronos_memory_entity_resolution::{EntityGraph, EntityType};
use chronos_memory_sessions::SessionProjection;
use chronos_memory_state::ProjectedStatePayload;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Error types for the Reflection Engine.
#[derive(Debug, thiserror::Error)]
pub enum ReflectionError {
    #[error("Parse error on ChronosState payload: {0}")]
    PayloadParseError(String),
}

/// The structured detail payload serialized inside the outcome_evaluation field of ChronosReflection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningReflectionPayload {
    pub reflection_id: String,
    pub timestamp: DateTime<Utc>,
    pub confidence: u8,
    pub evidence: Vec<String>,
    pub provenance_ids: Vec<String>,
    pub explanation: String,
}

pub struct ReflectionEngine;

impl ReflectionEngine {
    /// Evaluates ChronosState and dependencies to generate a factual reflection.
    pub fn reflect(
        state: &chronos_core::ChronosState,
        graph: &EntityGraph,
        sessions: &SessionProjection,
    ) -> Result<ChronosReflection, ReflectionError> {
        let payload: ProjectedStatePayload = serde_json::from_value(state.payload.clone())
            .map_err(|e| ReflectionError::PayloadParseError(e.to_string()))?;

        let mut evidence = Vec::new();
        let mut provenance_ids = payload.provenance_event_ids.clone();
        
        if let Some(ref active_id) = payload.active_session_id {
            provenance_ids.push(active_id.clone());
        }

        // 1. Detect Interrupted Sessions
        // Find closed sessions and see if their termination matches normal decay
        for session in sessions.sessions().values() {
            if session.is_closed {
                // If a session duration was > 300s and closed due to inactivity
                if session.duration > 300 {
                    evidence.push(format!("Session {} closed after {} seconds of activity", session.session_id, session.duration));
                }
            }
        }

        // 2. Detect Active Focus Areas
        if let Some(sess) = sessions.latest().filter(|s| !s.is_closed) {
            if !sess.repository_ids.is_empty() {
                for repo_id in &sess.repository_ids {
                    if let Some(entity) = graph.get_entity(repo_id) {
                        let name = entity.properties.get("name").map(|s| s.as_str()).unwrap_or("unknown");
                        evidence.push(format!("Active focus is dedicated to repository '{}'", name));
                    }
                }
            }
        }

        // 3. Detect Stalled Projects
        // Find projects that are not active in any current session
        let active_projects: HashSet<String> = sessions.latest()
            .map(|s| s.project_ids.clone())
            .unwrap_or_default();

        for entity in graph.entities().values() {
            if entity.entity_type == EntityType::Project && !active_projects.contains(&entity.id) {
                let name = entity.properties.get("name").map(|s| s.as_str()).unwrap_or("unknown");
                // Check if last event provenance in project is older than 24h
                let is_stalled = sessions.latest()
                    .map(|s| (Utc::now() - s.end_timestamp).num_seconds() > 86400)
                    .unwrap_or(true);
                    
                if is_stalled {
                    evidence.push(format!("Project '{}' is stalled (no active session referencing it)", name));
                }
            }
        }

        // Construct explanation
        let explanation = if evidence.is_empty() {
            "System is idle. No significant context changes or anomalies detected.".to_string()
        } else {
            format!("Detected {} observations regarding session timelines and focus states.", evidence.len())
        };

        // Aggregated confidence rating
        let confidence = (payload.aggregated_confidence * 100.0) as u8;

        let reflection_payload = ReasoningReflectionPayload {
            reflection_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            confidence,
            evidence,
            provenance_ids,
            explanation: explanation.clone(),
        };

        let outcome_str = serde_json::to_string(&reflection_payload).unwrap_or_default();

        // Create ChronosReflection mapping to frozen core object
        let reflection = ChronosReflection::new(
            None,
            None,
            outcome_str,
            0, // confidence delta default
        );

        Ok(reflection)
    }
}

use std::collections::HashSet;

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_core::ChronosEvent;
    use chronos_memory_entity_resolution::EntityResolver;
    use chronos_memory_sessions::SessionEngine;
    use chronos_memory_state::StateProjector;
    use serde_json::json;

    #[test]
    fn test_reflection_engine_rules() {
        let mut resolver = EntityResolver::new();
        let disc_event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos" }),
        );
        resolver.process_event(&disc_event).unwrap();

        // Create a stalled project via commit event
        let commit_event = ChronosEvent::new(
            "GitCommitCreated",
            "GitAdapter",
            json!({
                "repository_path": "/workspace/chronos",
                "source_payload": { "message": "First commit" }
            }),
        );
        resolver.process_event(&commit_event).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc_event).unwrap();
        session_engine.process_event(&commit_event).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());
        
        let reflection = ReflectionEngine::reflect(&state, &graph, session_engine.projection()).unwrap();
        
        // Deserialize outcome evaluation
        let payload: ReasoningReflectionPayload = serde_json::from_str(&reflection.outcome_evaluation).unwrap();
        
        assert!(payload.confidence > 0);
        assert!(!payload.evidence.is_empty());
        assert!(payload.explanation.contains("observations"));
    }

    #[test]
    fn test_reflection_determinism() {
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

        let state = StateProjector::project(&graph, session_engine.projection());

        let ref1 = ReflectionEngine::reflect(&state, &graph, session_engine.projection()).unwrap();
        let ref2 = ReflectionEngine::reflect(&state, &graph, session_engine.projection()).unwrap();

        let p1: ReasoningReflectionPayload = serde_json::from_str(&ref1.outcome_evaluation).unwrap();
        let p2: ReasoningReflectionPayload = serde_json::from_str(&ref2.outcome_evaluation).unwrap();

        assert_eq!(p1.confidence, p2.confidence);
        assert_eq!(p1.evidence, p2.evidence);
        assert_eq!(p1.explanation, p2.explanation);
    }
}
