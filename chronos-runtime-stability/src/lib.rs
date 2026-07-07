//! # Chronos CRLE Runtime Hardening & Load Stability Layer (CRLE-HLSL)
//!
//! HLSL provides tick stability guards, memory cache constraints, backpressure
//! throttling, and long-run degradation analysis to sustain continuous loops.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemDegradationDetectedPayload {
    pub average_tick_duration_ms: u64,
    pub queue_backlog_size: u64,
    pub degradation_factor: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemPerformanceDriftPayload {
    pub drift_score: f64,
    pub details: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct StabilityMetrics {
    pub average_tick_duration_ms: u64,
    pub backlog_size: u64,
    pub health_score: f64,
}

pub struct StabilityController {
    pub metrics: Arc<Mutex<StabilityMetrics>>,
    pub max_tick_limit_ms: u64,
}

impl StabilityController {
    pub fn new(max_tick_limit_ms: u64) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(StabilityMetrics {
                average_tick_duration_ms: 10,
                backlog_size: 0,
                health_score: 1.0,
            })),
            max_tick_limit_ms,
        }
    }

    /// Evaluates pipeline metrics and issues self-stabilization / degradation alert events.
    pub fn check_stability(&self, current_tick_ms: u64, backlog: u64) -> Vec<ChronosEvent> {
        let mut stability_events = Vec::new();
        let mut m = self.metrics.lock().unwrap();

        // Rolling average update
        m.average_tick_duration_ms = (m.average_tick_duration_ms * 9 + current_tick_ms) / 10;
        m.backlog_size = backlog;

        if m.average_tick_duration_ms > self.max_tick_limit_ms || backlog > 100 {
            let duration_degradation = (m.average_tick_duration_ms as f64 / self.max_tick_limit_ms as f64).min(2.0);
            let backlog_degradation = if backlog > 100 { 1.0 + (backlog - 100) as f64 / 100.0 } else { 1.0 };
            let degradation = duration_degradation.max(backlog_degradation);
            
            m.health_score = 1.0 - (degradation - 1.0).max(0.0);

            stability_events.push(ChronosEvent::new(
                "SystemDegradationDetected",
                "StabilityController",
                serde_json::to_value(SystemDegradationDetectedPayload {
                    average_tick_duration_ms: m.average_tick_duration_ms,
                    queue_backlog_size: backlog,
                    degradation_factor: degradation,
                    timestamp: Utc::now(),
                }).unwrap(),
            ));

            stability_events.push(ChronosEvent::new(
                "SystemPerformanceDrift",
                "StabilityController",
                serde_json::to_value(SystemPerformanceDriftPayload {
                    drift_score: (degradation - 1.0).max(0.0),
                    details: format!("Degradation detected: Tick avg ({}ms), backlog ({})", m.average_tick_duration_ms, backlog),
                    timestamp: Utc::now(),
                }).unwrap(),
            ));
        }

        stability_events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stability_degradation_alerting() {
        let controller = StabilityController::new(50); // 50ms budget

        // 1. Healthy tick test
        let mut events = controller.check_stability(20, 10);
        assert!(events.is_empty());

        // 2. Unhealthy tick triggers degradation alert
        events = controller.check_stability(80, 150);
        assert!(events.iter().any(|e| e.event_type == "SystemDegradationDetected"));
        assert!(events.iter().any(|e| e.event_type == "SystemPerformanceDrift"));

        let m = controller.metrics.lock().unwrap();
        assert!(m.health_score < 1.0);
    }
}
