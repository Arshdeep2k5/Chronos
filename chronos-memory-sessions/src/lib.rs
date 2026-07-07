//! # Chronos Cognitive Session Engine
//!
//! Groups raw events and knowledge entities into deterministic, chronological Cognitive Sessions.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use chronos_memory_entity_resolution::{EntityGraph, EntityType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Error types for the Session Engine.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Boundary calculation error: {0}")]
    BoundaryError(String),
}

/// Represents a contiguous block of focused human activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSession {
    pub session_id: String,
    pub start_timestamp: DateTime<Utc>,
    pub end_timestamp: DateTime<Utc>,
    pub duration: i64, // in seconds
    pub confidence: f64,
    pub entity_ids: HashSet<String>,
    pub repository_ids: HashSet<String>,
    pub artifact_ids: HashSet<String>,
    pub project_ids: HashSet<String>,
    pub source_event_ids: Vec<String>,
    pub resumes_session_id: Option<String>,
    pub is_closed: bool,
}

impl CognitiveSession {
    pub fn new(start: DateTime<Utc>, event_id: String, resumes: Option<String>) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            start_timestamp: start,
            end_timestamp: start,
            duration: 0,
            confidence: 1.0,
            entity_ids: HashSet::new(),
            repository_ids: HashSet::new(),
            artifact_ids: HashSet::new(),
            project_ids: HashSet::new(),
            source_event_ids: vec![event_id],
            resumes_session_id: resumes,
            is_closed: false,
        }
    }

    /// Extends the session boundary with a new event.
    pub fn extend(&mut self, timestamp: DateTime<Utc>, event_id: String) {
        if timestamp > self.end_timestamp {
            self.end_timestamp = timestamp;
            self.duration = (self.end_timestamp - self.start_timestamp).num_seconds();
        }
        if !self.source_event_ids.contains(&event_id) {
            self.source_event_ids.push(event_id);
        }
    }
}

/// Analyzes events to identify start and end session boundaries.
pub struct SessionBoundaryDetector {
    inactivity_threshold_sec: i64,
    last_event_time: Option<DateTime<Utc>>,
}

impl SessionBoundaryDetector {
    pub fn new(inactivity_threshold_sec: i64) -> Self {
        Self {
            inactivity_threshold_sec,
            last_event_time: None,
        }
    }

    /// Evaluates if an incoming event marks the start of a new session boundary.
    pub fn is_new_session(&mut self, event: &ChronosEvent) -> (bool, Option<i64>) {
        let timestamp = event.timestamp;
        
        // Explicit start indicator
        if event.event_type == "GitBranchSwitched" || event.event_type == "GitRepositoryDiscovered" {
            if self.last_event_time.is_none() {
                self.last_event_time = Some(timestamp);
                return (true, None);
            }
        }

        if let Some(last_time) = self.last_event_time {
            let diff = (timestamp - last_time).num_seconds();
            self.last_event_time = Some(timestamp);
            
            if diff > self.inactivity_threshold_sec {
                (true, Some(diff))
            } else {
                (false, Some(diff))
            }
        } else {
            self.last_event_time = Some(timestamp);
            (true, None)
        }
    }

    /// Resets the detector state.
    pub fn reset(&mut self) {
        self.last_event_time = None;
    }
}

/// Materialized collection of resolved sessions.
#[derive(Debug, Clone, Default)]
pub struct SessionProjection {
    sessions: HashMap<String, CognitiveSession>,
    ordered_ids: Vec<String>,
}

impl SessionProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, session: CognitiveSession) {
        self.ordered_ids.push(session.session_id.clone());
        self.sessions.insert(session.session_id.clone(), session);
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut CognitiveSession> {
        self.sessions.get_mut(id)
    }

    pub fn latest(&self) -> Option<&CognitiveSession> {
        self.ordered_ids.last().and_then(|id| self.sessions.get(id))
    }

    pub fn latest_mut(&mut self) -> Option<&mut CognitiveSession> {
        if let Some(id) = self.ordered_ids.last().cloned() {
            self.sessions.get_mut(&id)
        } else {
            None
        }
    }

    pub fn sessions(&self) -> &HashMap<String, CognitiveSession> {
        &self.sessions
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
        self.ordered_ids.clear();
    }
}

/// Helper that links sessions to related Knowledge Graph nodes.
pub struct SessionGraphLinker;

impl SessionGraphLinker {
    /// Links the active session to related entities in the graph based on event metadata.
    pub fn link(session: &mut CognitiveSession, event: &ChronosEvent, graph: &EntityGraph) {
        // Extract paths and map them to entity IDs
        if let Some(repo_path) = event.payload.get("repository_path").and_then(|v| v.as_str()) {
            for entity in graph.entities().values() {
                if entity.entity_type == EntityType::Repository && entity.properties.get("path").map(|s| s.as_str()) == Some(repo_path) {
                    session.repository_ids.insert(entity.id.clone());
                    session.entity_ids.insert(entity.id.clone());
                }
            }
        }

        // Link files
        if let Some(files) = event.payload.get("source_payload").and_then(|v| v.get("files")).and_then(|v| v.as_array()) {
            for val in files {
                if let Some(file_path) = val.as_str() {
                    for entity in graph.entities().values() {
                        if entity.entity_type == EntityType::File && entity.properties.get("path").map(|s| s.as_str()) == Some(file_path) {
                            session.artifact_ids.insert(entity.id.clone());
                            session.entity_ids.insert(entity.id.clone());
                        }
                    }
                }
            }
        }

        // Link projects
        for entity in graph.entities().values() {
            if entity.entity_type == EntityType::Project {
                // If this project tracks a repository that we have linked, link the project
                for repo_id in &session.repository_ids {
                    if graph.relationships().iter().any(|r| r.source_id == entity.id && r.target_id == *repo_id && r.relation_type == "tracks_repo") {
                        session.project_ids.insert(entity.id.clone());
                        session.entity_ids.insert(entity.id.clone());
                    }
                }
            }
        }
    }
}

/// The orchestrating engine executing boundaries and linking events.
pub struct SessionEngine {
    projection: SessionProjection,
    detector: SessionBoundaryDetector,
    graph: EntityGraph,
}

impl SessionEngine {
    pub fn new(inactivity_threshold_sec: i64, graph: EntityGraph) -> Self {
        Self {
            projection: SessionProjection::new(),
            detector: SessionBoundaryDetector::new(inactivity_threshold_sec),
            graph,
        }
    }

    /// Processes a single event, managing session boundaries and linkages.
    pub fn process_event(&mut self, event: &ChronosEvent) -> Result<Vec<ChronosEvent>, SessionError> {
        let mut side_effects = Vec::new();
        let (is_new, gap) = self.detector.is_new_session(event);

        if is_new {
            // Close active session if it exists
            if let Some(active) = self.projection.latest_mut() {
                if !active.is_closed {
                    active.is_closed = true;
                    side_effects.push(ChronosEvent::new(
                        "SessionClosed",
                        "SessionEngine",
                        serde_json::json!({ "session_id": active.session_id }),
                    ));
                }
            }

            // Determine if this is a resurrection (resumes chain)
            let resumes_id = if let Some(g) = gap {
                // Resurrection boundary logic: gap <= 4 * threshold is a continuation link
                if g <= 4 * self.detector.inactivity_threshold_sec {
                    self.projection.latest().map(|s| s.session_id.clone())
                } else {
                    None
                }
            } else {
                None
            };

            let mut new_session = CognitiveSession::new(event.timestamp, event.id.clone(), resumes_id.clone());
            SessionGraphLinker::link(&mut new_session, event, &self.graph);
            
            let id = new_session.session_id.clone();
            self.projection.add(new_session);

            if resumes_id.is_some() {
                side_effects.push(ChronosEvent::new(
                    "SessionResumed",
                    "SessionEngine",
                    serde_json::json!({ "session_id": id, "resumes_session_id": resumes_id }),
                ));
            } else {
                side_effects.push(ChronosEvent::new(
                    "SessionCreated",
                    "SessionEngine",
                    serde_json::json!({ "session_id": id }),
                ));
            }
        } else {
            // Extend existing session
            if let Some(active) = self.projection.latest_mut() {
                active.extend(event.timestamp, event.id.clone());
                SessionGraphLinker::link(active, event, &self.graph);
                
                side_effects.push(ChronosEvent::new(
                    "SessionUpdated",
                    "SessionEngine",
                    serde_json::json!({ "session_id": active.session_id }),
                ));
            }
        }

        Ok(side_effects)
    }

    /// Rebuilds the entire session timeline deterministically from an event replay.
    pub fn replay(&mut self, events: &[ChronosEvent]) -> Result<(), SessionError> {
        self.projection.clear();
        self.detector.reset();
        for event in events {
            self.process_event(event)?;
        }
        Ok(())
    }

    pub fn projection(&self) -> &SessionProjection {
        &self.projection
    }
}

/// A read-only interface to query completed and active sessions.
pub struct SessionRepository {
    engine: std::sync::Arc<std::sync::Mutex<SessionEngine>>,
}

impl SessionRepository {
    pub fn new(engine: std::sync::Arc<std::sync::Mutex<SessionEngine>>) -> Self {
        Self { engine }
    }

    pub fn list_sessions(&self) -> Vec<CognitiveSession> {
        let lock = self.engine.lock().unwrap();
        lock.projection().sessions().values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_memory_entity_resolution::EntityResolver;
    use serde_json::json;

    fn setup_entity_graph() -> EntityGraph {
        let mut resolver = EntityResolver::new();
        // Setup a mock Repository entity
        let event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos" }),
        );
        resolver.process_event(&event).unwrap();
        resolver.graph().clone()
    }

    #[test]
    fn test_session_creation() {
        let graph = setup_entity_graph();
        let mut engine = SessionEngine::new(10, graph);

        let event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos" }),
        );

        let effects = engine.process_event(&event).unwrap();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].event_type, "SessionCreated");

        let projection = engine.projection();
        assert_eq!(projection.sessions().len(), 1);
        
        let session = projection.latest().unwrap();
        assert!(session.repository_ids.len() >= 1);
        assert_eq!(session.duration, 0);
    }

    #[test]
    fn test_session_continuation() {
        let graph = setup_entity_graph();
        let mut engine = SessionEngine::new(10, graph); // 10 sec threshold

        let t1 = Utc::now();
        let mut e1 = ChronosEvent::new("Activity", "Src", json!({}));
        e1.timestamp = t1;

        let mut e2 = ChronosEvent::new("Activity", "Src", json!({}));
        e2.timestamp = t1 + chrono::Duration::seconds(5); // under 10 sec threshold

        engine.process_event(&e1).unwrap();
        let effects = engine.process_event(&e2).unwrap();

        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].event_type, "SessionUpdated");

        let session = engine.projection().latest().unwrap();
        assert_eq!(session.duration, 5);
        assert!(!session.is_closed);
    }

    #[test]
    fn test_session_resurrection() {
        let graph = setup_entity_graph();
        let mut engine = SessionEngine::new(10, graph);

        let t1 = Utc::now();
        let mut e1 = ChronosEvent::new("Activity", "Src", json!({}));
        e1.timestamp = t1;

        let mut e2 = ChronosEvent::new("Activity", "Src", json!({}));
        e2.timestamp = t1 + chrono::Duration::seconds(25); // Exceeds 10s threshold, under 40s (4 * threshold) limit

        engine.process_event(&e1).unwrap();
        let effects = engine.process_event(&e2).unwrap();

        // Should close first session and resurrect/resume in a new linked session
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0].event_type, "SessionClosed");
        assert_eq!(effects[1].event_type, "SessionResumed");

        let sessions = engine.projection();
        assert_eq!(sessions.sessions().len(), 2);

        let current = sessions.latest().unwrap();
        assert!(current.resumes_session_id.is_some());
    }

    #[test]
    fn test_replay_determinism() {
        let graph = setup_entity_graph();
        let mut engine = SessionEngine::new(10, graph);

        let t1 = Utc::now();
        let mut e1 = ChronosEvent::new("A1", "S", json!({}));
        e1.timestamp = t1;
        let mut e2 = ChronosEvent::new("A2", "S", json!({}));
        e2.timestamp = t1 + chrono::Duration::seconds(5);
        let mut e3 = ChronosEvent::new("A3", "S", json!({}));
        e3.timestamp = t1 + chrono::Duration::seconds(25);

        let events = vec![e1, e2, e3];
        
        engine.replay(&events).unwrap();
        let count1 = engine.projection().sessions().len();

        engine.replay(&events).unwrap();
        let count2 = engine.projection().sessions().len();

        assert_eq!(count1, count2);
        assert_eq!(count1, 2);
    }
}
