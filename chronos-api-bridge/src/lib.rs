//! # Chronos API Bridge — Library Root
//!
//! This crate implements the Axum HTTP API layer specified in `CHRONOS_UI_MIGRATION_PLAN.md`.
//! All endpoints consume real PCOS outputs — no mock data, no placeholder logic.
//!
//! ## Implemented Endpoints
//!
//! | Method | Path                                  | Source                     |
//! |--------|---------------------------------------|----------------------------|
//! | GET    | /api/events/stream                    | SQLite EventStore          |
//! | GET    | /api/state                            | StateProjector             |
//! | GET    | /api/reasoning/forecasts              | RiskEngine                 |
//! | GET    | /api/reasoning/diagnostics            | ReflectionEngine           |
//! | GET    | /api/execution/commitments/active     | CommitmentEngine           |
//! | POST   | /api/execution/generate-recovery-plan | CceEngine                  |
//! | POST   | /api/decision/simulate                | DecisionOrchestrator       |
//! | POST   | /api/execution/restore-workspace      | CceEngine + DecisionOrch.  |

pub mod handlers;
pub mod state;
pub mod router;
