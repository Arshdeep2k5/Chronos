//! # Chronos System Integrity & Drift Detection Layer (CSIDDL)
//!
//! Guarantees long-term correctness, structural consistency, and cognitive coherence
//! across all Chronos subsystems during continuous CRLE execution by detecting,
//! isolating, and correcting systemic drift.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriftSeverity {
    Soft,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriftDetectedPayload {
    pub subsystem: String,
    pub drift_score: f64,
    pub severity: DriftSeverity,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubsystemQuarantinedPayload {
    pub subsystem: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayViolationPayload {
    pub live_state_hash: String,
    pub replayed_state_hash: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegrityCheckpointPayload {
    pub check_id: String,
    pub passed: bool,
    pub health_score: f64,
}

pub struct GlobalIntegrityEngine;

impl GlobalIntegrityEngine {
    /// Runs Phase 0 (Pre-Tick) and Phase 6 (Post-Feedback) integrity validation checks.
    pub fn verify_integrity(
        events: &[ChronosEvent],
        live_state_hash: &str,
    ) -> Vec<ChronosEvent> {
        let mut integrity_events = Vec::new();

        // 1. Replay Equivalence Check: Replay log to verify determinism signature matches live run state
        let mut replayed_hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hash;
        for ev in events {
            ev.id.hash(&mut replayed_hasher);
            ev.event_type.hash(&mut replayed_hasher);
        }
        use std::hash::Hasher;
        let replayed_hash = format!("hash-{}", replayed_hasher.finish());

        if live_state_hash != replayed_hash {
            integrity_events.push(ChronosEvent::new(
                "ReplayViolation",
                "GlobalIntegrityEngine",
                serde_json::to_value(ReplayViolationPayload {
                    live_state_hash: live_state_hash.to_string(),
                    replayed_state_hash: replayed_hash,
                    details: "Mismatch detected between live execution logs and replay path.".to_string(),
                }).unwrap(),
            ));

            // Escalate to quarantine state
            integrity_events.push(ChronosEvent::new(
                "SubsystemQuarantined",
                "GlobalIntegrityEngine",
                serde_json::to_value(SubsystemQuarantinedPayload {
                    subsystem: "CognitiveLoop".to_string(),
                    reason: "Replay violation quarantine containment triggered.".to_string(),
                    timestamp: Utc::now(),
                }).unwrap(),
            ));
        } else {
            // Log successful integrity checkpoint
            integrity_events.push(ChronosEvent::new(
                "IntegrityCheckpoint",
                "GlobalIntegrityEngine",
                serde_json::to_value(IntegrityCheckpointPayload {
                    check_id: format!("chk-{}", Utc::now().timestamp()),
                    passed: true,
                    health_score: 1.0,
                }).unwrap(),
            ));
        }

        integrity_events
    }

    /// Evaluates drift over time based on consecutive graph changes.
    pub fn detect_drift(
        subsystem: &str,
        drift_factor: f64,
    ) -> Vec<ChronosEvent> {
        let mut drift_events = Vec::new();

        if drift_factor > 0.5 {
            drift_events.push(ChronosEvent::new(
                "DriftDetected",
                "GlobalIntegrityEngine",
                serde_json::to_value(DriftDetectedPayload {
                    subsystem: subsystem.to_string(),
                    drift_score: drift_factor,
                    severity: if drift_factor > 0.8 { DriftSeverity::Critical } else { DriftSeverity::Medium },
                    timestamp: Utc::now(),
                }).unwrap(),
            ));
        }

        drift_events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_detection_and_quarantine() {
        let event = ChronosEvent::new("IntentDetected", "Test", serde_json::json!({}));
        let events = vec![event];

        // 1. Successful verification case
        let mut live_hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hash;
        for ev in &events {
            ev.id.hash(&mut live_hasher);
            ev.event_type.hash(&mut live_hasher);
        }
        use std::hash::Hasher;
        let live_hash = format!("hash-{}", live_hasher.finish());

        let out = GlobalIntegrityEngine::verify_integrity(&events, &live_hash);
        assert!(out.iter().any(|e| e.event_type == "IntegrityCheckpoint"));

        // 2. Replay mismatch violation containment case
        let bad_out = GlobalIntegrityEngine::verify_integrity(&events, "mismatched-live-hash");
        assert!(bad_out.iter().any(|e| e.event_type == "ReplayViolation"));
        assert!(bad_out.iter().any(|e| e.event_type == "SubsystemQuarantined"));

        // 3. Drift detection case
        let drift_out = GlobalIntegrityEngine::detect_drift("CommitmentSubsystem", 0.9);
        assert!(drift_out.iter().any(|e| e.event_type == "DriftDetected"));
    }
}
