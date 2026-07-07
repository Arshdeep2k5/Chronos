//! # Chronos Commitment Canonicalization & Intent Extraction Layer
//!
//! Transforms raw reality signals and preliminary commitment events into stable,
//! deduplicated, and semantically grounded canonical commitments within the Chronos
//! event-sourced architecture.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    Explicit,
    Implicit,
    Behavioral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentSignal {
    pub intent_id: String,
    pub source: String,
    pub raw_content: String,
    pub signal_type: SignalType,
    pub timestamp: DateTime<Utc>,
    pub confidence_score: f64,
    pub context_metadata: HashMap<String, String>,
    pub provenance_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalCommitment {
    pub commitment: chronos_reasoning_commitments::Commitment,
    pub canonical_source_intents: Vec<IntentSignal>,
    pub merge_history: Vec<String>,
    pub deduplication_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDetectedPayload {
    pub intent: IntentSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentNormalizedPayload {
    pub intent_id: String,
    pub normalized_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMergedPayload {
    pub target_intent_id: String,
    pub merged_intent_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentCanonicalizedPayload {
    pub canonical_commitment: CanonicalCommitment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentDeDuplicatedPayload {
    pub commitment_id: String,
    pub deduplicated_with_id: String,
    pub trace: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CanonicalState {
    pub intents: HashMap<String, IntentSignal>,
    pub canonical_commitments: HashMap<String, CanonicalCommitment>,
}

impl CanonicalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_event(&mut self, event: &ChronosEvent) {
        match event.event_type.as_str() {
            "IntentDetected" => {
                if let Ok(payload) = serde_json::from_value::<IntentDetectedPayload>(event.payload.clone()) {
                    self.intents.insert(payload.intent.intent_id.clone(), payload.intent);
                }
            }
            "IntentNormalized" => {
                if let Ok(payload) = serde_json::from_value::<IntentNormalizedPayload>(event.payload.clone()) {
                    if let Some(intent) = self.intents.get_mut(&payload.intent_id) {
                        intent.raw_content = payload.normalized_content;
                        intent.provenance_event_ids.push(event.id.clone());
                    }
                }
            }
            "IntentMerged" => {
                if let Ok(payload) = serde_json::from_value::<IntentMergedPayload>(event.payload.clone()) {
                    if let Some(merged) = self.intents.remove(&payload.merged_intent_id) {
                        if let Some(target) = self.intents.get_mut(&payload.target_intent_id) {
                            target.provenance_event_ids.extend(merged.provenance_event_ids);
                            target.confidence_score = (target.confidence_score + 0.05).min(1.0);
                            target.provenance_event_ids.push(event.id.clone());
                        }
                    }
                }
            }
            "CommitmentCanonicalized" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentCanonicalizedPayload>(event.payload.clone()) {
                    self.canonical_commitments.insert(
                        payload.canonical_commitment.commitment.commitment_id.clone(),
                        payload.canonical_commitment,
                    );
                }
            }
            "CommitmentDeDuplicated" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentDeDuplicatedPayload>(event.payload.clone()) {
                    if let Some(mut target) = self.canonical_commitments.remove(&payload.commitment_id) {
                        if let Some(dest) = self.canonical_commitments.get_mut(&payload.deduplicated_with_id) {
                            dest.merge_history.push(payload.commitment_id);
                            dest.deduplication_trace.push(payload.trace);
                            dest.canonical_source_intents.append(&mut target.canonical_source_intents);
                            dest.commitment.confidence = (dest.commitment.confidence + 0.1).min(1.0);
                            dest.commitment.provenance.push(event.id.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct CanonicalCommitmentBuilder;

impl CanonicalCommitmentBuilder {
    /// Normalizes a raw perception event into an intermediate IntentSignal representation.
    pub fn normalize_event(event: &ChronosEvent) -> Option<IntentSignal> {
        match event.event_type.as_str() {
            "BrowserUrlChanged" | "BrowserTabActivated" => {
                let url = event.payload.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let title = event.payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
                if url.contains("github.com") || url.contains("jira") || url.contains("linear") {
                    let mut meta = HashMap::new();
                    meta.insert("url".to_string(), url.to_string());
                    meta.insert("title".to_string(), title.to_string());
                    Some(IntentSignal {
                        intent_id: format!("intent-browser-{}", event.id),
                        source: "browser".to_string(),
                        raw_content: format!("Browse task context: {}", title),
                        signal_type: SignalType::Implicit,
                        timestamp: event.timestamp,
                        confidence_score: 0.7,
                        context_metadata: meta,
                        provenance_event_ids: vec![event.id.clone()],
                    })
                } else {
                    None
                }
            }
            "EditorFileOpened" => {
                let file_path = event.payload.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
                let mut meta = HashMap::new();
                meta.insert("file_path".to_string(), file_path.to_string());
                Some(IntentSignal {
                    intent_id: format!("intent-editor-{}", event.id),
                    source: "vscode".to_string(),
                    raw_content: format!("Work on file: {}", file_path),
                    signal_type: SignalType::Behavioral,
                    timestamp: event.timestamp,
                    confidence_score: 0.6,
                    context_metadata: meta,
                    provenance_event_ids: vec![event.id.clone()],
                })
            }
            "GitCommitCreated" => {
                let message = event.payload.get("message").and_then(|v| v.as_str()).unwrap_or("");
                let mut meta = HashMap::new();
                meta.insert("commit_message".to_string(), message.to_string());
                Some(IntentSignal {
                    intent_id: format!("intent-git-{}", event.id),
                    source: "git".to_string(),
                    raw_content: message.to_string(),
                    signal_type: SignalType::Explicit,
                    timestamp: event.timestamp,
                    confidence_score: 1.0,
                    context_metadata: meta,
                    provenance_event_ids: vec![event.id.clone()],
                })
            }
            "ManualContextIngested" => {
                let display_name = event.payload.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
                let mut meta = HashMap::new();
                meta.insert("display_name".to_string(), display_name.to_string());
                Some(IntentSignal {
                    intent_id: format!("intent-manual-{}", event.id),
                    source: "manual".to_string(),
                    raw_content: display_name.to_string(),
                    signal_type: SignalType::Explicit,
                    timestamp: event.timestamp,
                    confidence_score: 0.95,
                    context_metadata: meta,
                    provenance_event_ids: vec![event.id.clone()],
                })
            }
            _ => None,
        }
    }

    /// Evaluates new IntentSignals and returns new events for canonicalization and deduplication.
    pub fn process_new_intent(state: &CanonicalState, new_intent: IntentSignal) -> Vec<ChronosEvent> {
        let mut events = Vec::new();

        // 1. Emit IntentDetected
        events.push(ChronosEvent::new(
            "IntentDetected",
            "CanonicalizationEngine",
            serde_json::to_value(IntentDetectedPayload { intent: new_intent.clone() }).unwrap(),
        ));

        // 2. Check for deduplication / semantic equivalence with existing intents
        let mut merged = false;
        let mut target_id = String::new();

        for existing in state.intents.values() {
            let time_diff = (existing.timestamp - new_intent.timestamp).num_minutes().abs();
            let is_similar_content = existing.raw_content.to_lowercase() == new_intent.raw_content.to_lowercase()
                || (existing.raw_content.contains(&new_intent.raw_content) && new_intent.raw_content.len() > 5)
                || (new_intent.raw_content.contains(&existing.raw_content) && existing.raw_content.len() > 5);

            if existing.source == new_intent.source && is_similar_content && time_diff <= 30 {
                merged = true;
                target_id = existing.intent_id.clone();
                break;
            }
        }

        if merged {
            events.push(ChronosEvent::new(
                "IntentMerged",
                "CanonicalizationEngine",
                serde_json::to_value(IntentMergedPayload {
                    target_intent_id: target_id,
                    merged_intent_id: new_intent.intent_id,
                    timestamp: Utc::now(),
                }).unwrap(),
            ));
        } else {
            // Deterministic stable ID calculation
            let commitment_id = generate_stable_id(&new_intent.raw_content, &new_intent.source);
            let canonical = CanonicalCommitment {
                commitment: chronos_reasoning_commitments::Commitment {
                    commitment_id: commitment_id.clone(),
                    source: new_intent.source.clone(),
                    content: new_intent.raw_content.clone(),
                    status: chronos_reasoning_commitments::CommitmentStatus::Candidate,
                    created_at: new_intent.timestamp,
                    inferred_due_at: new_intent.context_metadata.get("due_at")
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    confidence: new_intent.confidence_score,
                    provenance: new_intent.provenance_event_ids.clone(),
                },
                canonical_source_intents: vec![new_intent],
                merge_history: Vec::new(),
                deduplication_trace: Vec::new(),
            };

            if state.canonical_commitments.contains_key(&commitment_id) {
                events.push(ChronosEvent::new(
                    "CommitmentDeDuplicated",
                    "CanonicalizationEngine",
                    serde_json::to_value(CommitmentDeDuplicatedPayload {
                        commitment_id: commitment_id.clone(),
                        deduplicated_with_id: commitment_id,
                        trace: "Stable ID match deduplication".to_string(),
                    }).unwrap(),
                ));
            } else {
                events.push(ChronosEvent::new(
                    "CommitmentCanonicalized",
                    "CanonicalizationEngine",
                    serde_json::to_value(CommitmentCanonicalizedPayload {
                        canonical_commitment: canonical,
                    }).unwrap(),
                ));
            }
        }

        events
    }

    /// Reconstructs the entire CanonicalState projection from an ordered event log.
    pub fn rebuild_intent_and_commitment_state(events: &[ChronosEvent]) -> CanonicalState {
        let mut state = CanonicalState::new();
        for event in events {
            state.apply_event(event);
        }
        state
    }
}

fn generate_stable_id(content: &str, source: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hash;
    content.hash(&mut hasher);
    source.hash(&mut hasher);
    use std::hash::Hasher;
    let hash = hasher.finish();
    format!("canonical-commitment-{}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_intent_normalization() {
        let event = ChronosEvent::new(
            "GitCommitCreated",
            "GitAdapter",
            json!({ "message": "feat: canonical commit logic" }),
        );
        let intent = CanonicalCommitmentBuilder::normalize_event(&event).unwrap();
        assert_eq!(intent.raw_content, "feat: canonical commit logic");
        assert_eq!(intent.signal_type, SignalType::Explicit);
    }

    #[test]
    fn test_intent_deduplication_and_merge() {
        let created_at = Utc::now();
        let event1 = ChronosEvent::new(
            "ManualContextIngested",
            "Manual",
            json!({ "display_name": "Obligation task A" }),
        );
        let mut intent1 = CanonicalCommitmentBuilder::normalize_event(&event1).unwrap();
        intent1.timestamp = created_at;

        let event2 = ChronosEvent::new(
            "ManualContextIngested",
            "Manual",
            json!({ "display_name": "Obligation task A" }),
        );
        let mut intent2 = CanonicalCommitmentBuilder::normalize_event(&event2).unwrap();
        // Within 30 minutes
        intent2.timestamp = created_at + chrono::Duration::minutes(10);
        intent2.intent_id = "intent-manual-event2".to_string();

        let mut state = CanonicalState::new();

        // Process first intent
        let evs1 = CanonicalCommitmentBuilder::process_new_intent(&state, intent1);
        for ev in &evs1 {
            state.apply_event(ev);
        }

        // Process second similar intent
        let evs2 = CanonicalCommitmentBuilder::process_new_intent(&state, intent2);
        for ev in &evs2 {
            state.apply_event(ev);
        }

        // The second intent should trigger IntentMerged
        assert!(evs2.iter().any(|e| e.event_type == "IntentMerged"));
        // Only 1 intent remains in state due to merge
        assert_eq!(state.intents.len(), 1);
    }

    #[test]
    fn test_replay_fidelity() {
        let event1 = ChronosEvent::new(
            "IntentDetected",
            "Test",
            json!({
                "intent": {
                    "intent_id": "test-id",
                    "source": "manual",
                    "raw_content": "Implement auth",
                    "signal_type": "Explicit",
                    "timestamp": Utc::now(),
                    "confidence_score": 0.95,
                    "context_metadata": {},
                    "provenance_event_ids": []
                }
            }),
        );

        let events = vec![event1];
        let state = CanonicalCommitmentBuilder::rebuild_intent_and_commitment_state(&events);
        assert!(state.intents.contains_key("test-id"));
    }
}
