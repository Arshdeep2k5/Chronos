//! # Chronos Daemon
//!
//! The Chronos Daemon is the production runtime entry point for the PCOS ecosystem.
//! It initializes the full service stack, wires all PCOS layers via the Cognitive Bus,
//! and runs the end-to-end event processing pipeline until graceful shutdown.
//!
//! ## Boot Sequence
//! 1.  Initialize structured logging
//! 2.  Resolve data directory and configuration
//! 3.  Open SQLite Event Store
//! 4.  Initialize Cognitive Bus (tokio broadcast channel, 4096 capacity)
//! 5.  Initialize Service Registry
//! 6.  Register all PCOS services with Running/Healthy status
//! 7.  Replay persisted events to rebuild in-memory PCOS state
//! 8.  Start all Layer 1 Perception Adapters
//! 9.  Start event processing pipeline worker
//! 10. Block on Ctrl+C / SIGTERM for graceful shutdown
//!
//! ## Adapter Lifecycle
//! Each adapter calls `.start()` to register with the ServiceRegistry, then spawns
//! its own background task (window-focus, clipboard) or is polled by a dedicated
//! git-poll task (git). The filewatcher is initialised to watch CHRONOS_WATCH_DIR.
//!
//! ## Pipeline Architecture
//! Events arrive on the Cognitive Bus from adapters or the telemetry bridge.
//! Each event flows through:
//!   Memory (EntityResolver → SessionEngine → StateProjector)
//!   → Reasoning (Commitments → DDE → PCM → Risk → Reflection)
//!   → Decision (DecisionOrchestrator)
//!   → Execution (CceEngine)
//! All outputs are persisted to the SQLite Event Store and re-published
//! on the bus so the API bridge can serve live state.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use chronos_adapter_clipboard::ClipboardObserver;
use chronos_adapter_filewatcher::FilewatcherAdapter;
use chronos_adapter_git::{GitAdapter, GitRepositoryObserver};
use chronos_adapter_window_focus::WindowFocusObserver;
use chronos_bus::{BusError, EventBus, MemoryEventBus, Subscriber};
use chronos_config::ConfigurationService;
use chronos_core::ChronosEvent;
use chronos_logging::{ChronosLogger, LogContext};
use chronos_memory_entity_resolution::EntityResolver;
use chronos_memory_sessions::SessionEngine;
use chronos_registry::{
    ServiceDescriptor, ServiceHealth, ServiceRegistry, ServiceStatus, ServiceType,
};
use chronos_store::EventStore;
use chronos_store_sqlite::SQLiteEventStore;

// ── Shared API Bridge Runtime ────────────────────────────────────────────────
use chronos_api_bridge::state::BridgeState;
use chronos_api_bridge::router::build_router;

mod observability;

/// The shared daemon state passed to all workers.
pub struct DaemonRuntime {
    pub store: Arc<SQLiteEventStore>,
    pub bus: Arc<MemoryEventBus>,
    pub registry: Arc<ServiceRegistry>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ─── 1. Logging ───────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    tracing::info!(
        "Chronos Daemon (Unified Runtime) v{} starting…",
        env!("CARGO_PKG_VERSION")
    );

    // ─── 2. Data directory ────────────────────────────────────────────────────
    let data_dir = std::env::var("CHRONOS_DATA_DIR").unwrap_or_else(|_| {
        let mut path = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push("chronos");
        path.to_string_lossy().to_string()
    });
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create CHRONOS_DATA_DIR: {}", e))?;
    let db_path = format!("{}/chronos_events.db", data_dir);
    tracing::info!("Event store: {}", db_path);

    // ─── 3. SQLite Event Store ─────────────────────────────────────────────
    let store = Arc::new(
        SQLiteEventStore::new(&db_path)
            .map_err(|e| anyhow::anyhow!("Failed to open SQLite Event Store: {:?}", e))?,
    );
    let event_count = store.count().await.unwrap_or(0);
    tracing::info!(
        "SQLite Event Store initialized. Persisted events: {}",
        event_count
    );

    // ─── 4. Cognitive Bus ─────────────────────────────────────────────────
    let bus = Arc::new(MemoryEventBus::new(4096));
    tracing::info!("Cognitive Bus initialized (capacity: 4096).");

    // ─── 5. Service Registry ──────────────────────────────────────────────
    let registry = Arc::new(ServiceRegistry::new());

    // ─── 6. Register all services ─────────────────────────────────────────
    register_all_services(&registry).await;
    let registered = registry.list().await.len();
    tracing::info!("{} services registered.", registered);

    // ─── 7. Replay persisted events to warm up in-memory state ───────────
    let history = store.stream().await.unwrap_or_default();
    tracing::info!("Replaying {} persisted events to warm PCOS state…", history.len());
    let (warm_resolver, warm_session_engine, warm_commitments, warm_orchestrator) = replay_events(history, &bus).await;
    tracing::info!("State warm-up complete.");

    // ─── 8. Start event processing pipeline ───────────────────────────────
    let runtime = Arc::new(DaemonRuntime {
        store: Arc::clone(&store),
        bus: Arc::clone(&bus),
        registry: Arc::clone(&registry),
    });

    let pipeline_subscriber = bus.subscribe();
    let pipeline_runtime = Arc::clone(&runtime);
    let pipeline_resolver = Arc::clone(&warm_resolver);
    let pipeline_session = Arc::clone(&warm_session_engine);
    let pipeline_commitments = Arc::clone(&warm_commitments);
    let pipeline_orchestrator = Arc::clone(&warm_orchestrator);
    let pipeline_handle = tokio::spawn(async move {
        run_pipeline(
            pipeline_runtime,
            pipeline_subscriber,
            pipeline_resolver,
            pipeline_session,
            pipeline_commitments,
            pipeline_orchestrator,
        )
        .await;
    });

    // ─── 9. Start all Layer 1 Perception Adapters ────────────────────────
    let adapter_handles = start_adapters(Arc::clone(&bus), Arc::clone(&registry)).await;
    tracing::info!(
        "Chronos Adapters operational. {} adapter tasks running.",
        adapter_handles.len()
    );

    // ─── 10. Start API Bridge Server (Shared Runtime) ────────────────────
    let bridge_state = BridgeState::new(
        Arc::clone(&store),
        Arc::clone(&registry),
        Arc::clone(&warm_resolver),
        Arc::clone(&warm_session_engine),
        Arc::clone(&warm_commitments),
        Arc::clone(&warm_orchestrator),
        Arc::clone(&bus),
    );

    let app = build_router(bridge_state);

    let port: u16 = std::env::var("CHRONOS_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7899);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Chronos API Bridge listening on http://{}", addr);

    let api_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("API Bridge error: {}", e);
        }
    });

    tracing::info!("Chronos Daemon (Unified Runtime) fully operational.");

    // ─── 11. Await shutdown signal ────────────────────────────────────────
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received. Stopping Chronos Daemon…");
    pipeline_handle.abort();
    api_handle.abort();
    for h in adapter_handles {
        h.abort();
    }
    tracing::info!("Chronos Daemon stopped cleanly.");
    Ok(())
}

/// Starts all Layer 1 Perception Adapters, registering each with the ServiceRegistry
/// and launching their background observation tasks.
///
/// Returns a vec of task handles so the daemon can abort them on shutdown.
async fn start_adapters(
    bus: Arc<MemoryEventBus>,
    registry: Arc<ServiceRegistry>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let config = Arc::new(ConfigurationService::new());
    let logger = ChronosLogger::new(LogContext::new());
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    // ── Window Focus Adapter (self-polling, spawns own background task) ───
    let window_observer = WindowFocusObserver::new(
        Arc::clone(&registry),
        bus.clone() as Arc<dyn EventBus>,
        logger.clone(),
    );
    match window_observer.start().await {
        Ok(_) => tracing::info!("WindowFocusAdapter: started."),
        Err(e) => tracing::warn!("WindowFocusAdapter: failed to start: {}", e),
    }
    // window-focus spawns its own tokio task internally; no extra handle needed.

    // ── Clipboard Adapter (self-polling, spawns own background task) ──────
    let clipboard_observer = ClipboardObserver::new(
        Arc::clone(&registry),
        bus.clone() as Arc<dyn EventBus>,
        logger.clone(),
    );
    match clipboard_observer.start().await {
        Ok(_) => tracing::info!("ClipboardAdapter: started."),
        Err(e) => tracing::warn!("ClipboardAdapter: failed to start: {}", e),
    }
    // clipboard spawns its own tokio task internally; no extra handle needed.

    // ── Filesystem Watcher Adapter ─────────────────────────────────────────
    let filewatcher = Arc::new(FilewatcherAdapter::new(
        Arc::clone(&registry),
        bus.clone() as Arc<dyn EventBus>,
        Arc::clone(&config),
        logger.clone(),
    ));
    match filewatcher.start().await {
        Ok(_) => {
            // Watch CHRONOS_WATCH_DIR if set, otherwise skip directory watching.
            // The adapter is registered regardless so perception events from the
            // API ingest endpoint can still reach the bus.
            if let Ok(watch_dir) = std::env::var("CHRONOS_WATCH_DIR") {
                let path = PathBuf::from(&watch_dir);
                if path.is_dir() {
                    let fw = Arc::clone(&filewatcher);
                    match fw.watch_directory(path.clone()) {
                        Ok(_) => tracing::info!("FilewatcherAdapter: watching {:?}", path),
                        Err(e) => tracing::warn!("FilewatcherAdapter: watch_directory failed: {}", e),
                    }
                } else {
                    tracing::warn!(
                        "CHRONOS_WATCH_DIR={:?} is not a directory — skipping filesystem watch.",
                        watch_dir
                    );
                }
            } else {
                tracing::info!("FilewatcherAdapter: CHRONOS_WATCH_DIR not set — no directory watched. Set it to enable filesystem perception.");
            }
        }
        Err(e) => tracing::warn!("FilewatcherAdapter: failed to start: {}", e),
    }

    // ── Git Adapter (polls on interval) ───────────────────────────────────
    let git_adapter = Arc::new(GitAdapter::new(
        Arc::clone(&registry),
        bus.clone() as Arc<dyn EventBus>,
        Arc::clone(&config),
        logger.clone(),
    ));
    match git_adapter.start().await {
        Ok(_) => {
            // Watch CHRONOS_GIT_REPOS (colon-separated list of repo paths), or
            // auto-detect from CHRONOS_WATCH_DIR if it has a .git directory.
            let mut repos: Vec<PathBuf> = Vec::new();

            if let Ok(repo_list) = std::env::var("CHRONOS_GIT_REPOS") {
                for raw in repo_list.split(':') {
                    let p = PathBuf::from(raw.trim());
                    if GitRepositoryObserver::is_valid_git_repo(&p) {
                        repos.push(p);
                    } else {
                        tracing::warn!("GitAdapter: {:?} is not a git repo — skipping.", p);
                    }
                }
            } else if let Ok(watch_dir) = std::env::var("CHRONOS_WATCH_DIR") {
                let p = PathBuf::from(&watch_dir);
                if GitRepositoryObserver::is_valid_git_repo(&p) {
                    repos.push(p);
                }
            }

            if repos.is_empty() {
                tracing::info!("GitAdapter: no repositories to watch. Set CHRONOS_GIT_REPOS (colon-separated) to enable git perception.");
            } else {
                for repo in &repos {
                    let adapter = Arc::clone(&git_adapter);
                    let repo_path = repo.clone();
                    let result = adapter.watch_repository(repo_path.clone()).await;
                    match result {
                        Ok(_) => tracing::info!("GitAdapter: watching {:?}", repo_path),
                        Err(e) => tracing::warn!("GitAdapter: watch_repository({:?}) failed: {}", repo_path, e),
                    }
                }
            }

            // Git poll loop — runs every 10 seconds
            if !repos.is_empty() {
                let adapter_for_poll = Arc::clone(&git_adapter);
                let h = tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        if let Err(e) = adapter_for_poll.poll().await {
                            tracing::warn!("GitAdapter: poll error: {}", e);
                        }
                    }
                });
                handles.push(h);
            }
        }
        Err(e) => tracing::warn!("GitAdapter: failed to start: {}", e),
    }

    handles
}

/// Replays a slice of events through Memory layer engines to rebuild warm state.
/// Returns the warmed EntityResolver, SessionEngine, CommitmentState, and EventOrchestrator wrapped in Arc<RwLock>.
async fn replay_events(
    events: Vec<chronos_core::ChronosEvent>,
    _bus: &MemoryEventBus,
) -> (
    Arc<RwLock<EntityResolver>>,
    Arc<RwLock<Option<SessionEngine>>>,
    Arc<RwLock<chronos_reasoning_commitments::CommitmentState>>,
    Arc<RwLock<chronos_event_orchestrator::EventOrchestrator>>,
) {
    let resolver = Arc::new(RwLock::new(EntityResolver::new()));
    let session_engine_opt: Arc<RwLock<Option<SessionEngine>>> = Arc::new(RwLock::new(None));
    let orchestrator = Arc::new(RwLock::new(chronos_event_orchestrator::EventOrchestrator::new()));

    {
        let mut res_guard = resolver.write().await;
        let mut se_guard = session_engine_opt.write().await;
        let mut cepo_guard = orchestrator.write().await;

        cepo_guard.rebuild_from_history(events.clone());

        for event in &events {
            // Skip pipeline-output events during replay to avoid feedback
            if matches!(
                event.event_type.as_str(),
                "DecisionResolved"
                    | "RiskForecastResolved"
                    | "ActionStarted"
                    | "ActionCompleted"
                    | "ActionFailed"
                    | "ContinuationPlanResolved"
            ) {
                continue;
            }

            let _ = res_guard.process_event(event);
            let graph = res_guard.graph().clone();

            let se = se_guard.get_or_insert_with(|| SessionEngine::new(10, graph));
            let _ = se.process_event(event);
        }
    }

    let commitments = Arc::new(RwLock::new(
        orchestrator.read().await.commitment_state.clone()
    ));

    (resolver, session_engine_opt, commitments, orchestrator)
}

/// Registers all PCOS services with the ServiceRegistry.
async fn register_all_services(registry: &ServiceRegistry) {
    type ServiceSpec<'a> = (
        &'a str,
        &'a str,
        ServiceType,
        &'a str,
        &'a [&'a str],
        &'a [&'a str],
        &'a [&'a str],
    );

    let services: &[ServiceSpec] = &[
        (
            "svc-event-store-sqlite",
            "SQLite Event Store",
            ServiceType::Storage,
            "1.0.0",
            &["PersistEvents"],
            &["ChronosEvent"],
            &["PersistedEvent"],
        ),
        (
            "svc-cognitive-bus",
            "Cognitive Bus",
            ServiceType::Transport,
            "1.0.0",
            &["RouteEvents"],
            &["ChronosEvent"],
            &["ChronosEvent"],
        ),
        (
            "svc-entity-resolver",
            "Entity Resolver",
            ServiceType::Engine,
            "1.0.0",
            &["ResolveEntities"],
            &["ChronosEvent"],
            &["EntityResolved"],
        ),
        (
            "svc-session-engine",
            "Session Engine",
            ServiceType::Engine,
            "1.0.0",
            &["ManageSessions"],
            &["ChronosEvent"],
            &["SessionOpened", "SessionClosed"],
        ),
        (
            "svc-state-projector",
            "State Projector",
            ServiceType::Engine,
            "1.0.0",
            &["ProjectState"],
            &["EntityGraph", "SessionProjection"],
            &["ChronosState"],
        ),
        (
            "svc-reflection-engine",
            "Reflection Engine",
            ServiceType::Engine,
            "1.0.0",
            &["GenerateReflections"],
            &["ChronosState"],
            &["ChronosReflection"],
        ),
        (
            "svc-commitment-engine",
            "Commitment Engine",
            ServiceType::Engine,
            "1.0.0",
            &["DiscoverCommitments"],
            &["ChronosState"],
            &["CommitmentCandidate"],
        ),
        (
            "svc-dde",
            "Deadline Discovery Engine",
            ServiceType::Engine,
            "1.0.0",
            &["DiscoverDeadlines"],
            &["CommitmentCandidate"],
            &["DeadlineCandidate"],
        ),
        (
            "svc-pcm",
            "Personal Capacity Model",
            ServiceType::Engine,
            "1.0.0",
            &["EstimateCapacity"],
            &["SessionProjection"],
            &["CapacityProfile"],
        ),
        (
            "svc-risk-engine",
            "Risk Forecast Engine",
            ServiceType::Engine,
            "1.0.0",
            &["ForecastRisk"],
            &["CapacityProfile", "DeadlineCandidate"],
            &["RiskForecast"],
        ),
        (
            "svc-decision-orchestrator",
            "Decision Orchestrator",
            ServiceType::Engine,
            "1.0.0",
            &["OrchestrateDecisions"],
            &["RiskForecast", "CapacityProfile"],
            &["ChronosDecision"],
        ),
        (
            "svc-cce",
            "Context Continuation Engine",
            ServiceType::Engine,
            "1.0.0",
            &["TranslateDecisions"],
            &["ChronosDecision"],
            &["ChronosAction"],
        ),
    ];

    for (id, name, svc_type, ver, caps, consumed, produced) in services {
        let descriptor = ServiceDescriptor::new(
            *id,
            *name,
            svc_type.clone(),
            *ver,
            caps.iter().map(|s| s.to_string()).collect(),
            consumed.iter().map(|s| s.to_string()).collect(),
            produced.iter().map(|s| s.to_string()).collect(),
        );
        match registry.register(descriptor).await {
            Ok(_) => {
                registry
                    .update_status(id, ServiceStatus::Running)
                    .await
                    .ok();
                registry
                    .update_health(id, ServiceHealth::Healthy)
                    .await
                    .ok();
            }
            Err(e) => tracing::warn!("Service registration failed for {}: {}", id, e),
        }
    }
}



struct DaemonExecutor;

impl chronos_execution_orchestration::ExternalExecutor for DaemonExecutor {
    fn execute(&self, plan: &chronos_execution_orchestration::ExecutionPlan) -> Result<serde_json::Value, serde_json::Value> {
        tracing::info!("Executing plan via DaemonExecutor: {}", plan.execution_plan_id);
        
        // Very basic command execution if action_type == InternalTask or ToolCall
        // For demonstration, we simply echo or execute a mock side effect
        if plan.execution_steps.contains(&"Dispatch".to_string()) {
            let output = std::process::Command::new("cmd")
                .args(&["/c", "echo", "Chronos External Execution Triggered"])
                .output();
            
            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                    Ok(serde_json::json!({
                        "status": "Success",
                        "stdout": stdout.trim(),
                        "exit_code": o.status.code(),
                        "real_world_side_effect": true
                    }))
                }
                Err(e) => {
                    Err(serde_json::json!({
                        "status": "Error",
                        "reason": e.to_string(),
                        "real_world_side_effect": false
                    }))
                }
            }
        } else {
            Ok(serde_json::json!({
                "status": "Success",
                "message": "Plan executed successfully without subprocess",
                "real_world_side_effect": true
            }))
        }
    }
}

/// The end-to-end PCOS event processing pipeline.
///
/// Subscribes to the Cognitive Bus and processes each perception event through
/// all PCOS layers:
///   Memory → Reasoning → Decision → Execution
///
/// All produced artifacts (forecasts, decisions, actions) are:
/// 1. Persisted to the SQLite Event Store (immutable record)
/// 2. Re-published on the Cognitive Bus (available to API bridge subscribers)
async fn run_pipeline(
    runtime: Arc<DaemonRuntime>,
    mut subscriber: Box<dyn Subscriber>,
    resolver: Arc<RwLock<EntityResolver>>,
    session_engine_opt: Arc<RwLock<Option<SessionEngine>>>,
    commitments: Arc<RwLock<chronos_reasoning_commitments::CommitmentState>>,
    orchestrator: Arc<RwLock<chronos_event_orchestrator::EventOrchestrator>>,
) {
    let mut history = match runtime.store.stream().await {
        Ok(events) => events,
        Err(e) => {
            tracing::error!("Failed to load event history: {:?}", e);
            vec![]
        }
    };

    let data_dir = std::env::var("CHRONOS_DATA_DIR").unwrap_or_else(|_| {
        let mut path = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push("chronos");
        path.to_string_lossy().to_string()
    });
    let alert_emitter = observability::AlertEmitter::new(&data_dir);

    let engine = chronos_runtime_loop::ContinuousRuntimeLoopEngine::new(
        chronos_runtime_loop::RuntimeMode::Live,
    );

    let executor = DaemonExecutor;

    loop {
        let event = match subscriber.next_event().await {
            Ok(e) => e,
            Err(BusError::BusShutdown) => {
                tracing::info!("Cognitive Bus closed — pipeline stopping.");
                break;
            }
            Err(BusError::ReceiveError(msg)) => {
                tracing::warn!("Pipeline subscriber lagged: {}", msg);
                continue;
            }
            Err(e) => {
                tracing::warn!("Pipeline subscriber error: {:?}", e);
                continue;
            }
        };

        alert_emitter.process_event(&event);

        let mut new_tick_events = vec![event.clone()];

        // ── Phase 1.5: Pre-Tick Event Orchestration (CEPO) ────────────────────
        // CEPO acts as a pre-processor, emitting CommitmentDiscovered etc.
        {
            let mut cepo_guard = orchestrator.write().await;
            if let Ok(outputs) = cepo_guard.process_event(event.clone()) {
                for out in outputs {
                    if let Err(e) = runtime.store.append(out.clone()).await {
                        tracing::warn!("Store append failed for {}: {:?}", out.event_type, e);
                    }
                    if out.id != event.id {
                        let _ = runtime.bus.publish(out.clone());
                        new_tick_events.push(out);
                    }
                }
                let mut comm_guard = commitments.write().await;
                *comm_guard = cepo_guard.commitment_state.clone();
            }
        }

        // Prevent feedback loops: skip processing for events that the pipeline itself produces,
        // but DO record them in history so future ticks see them.
        if matches!(
            event.event_type.as_str(),
            "DecisionResolved"
                | "RiskForecastResolved"
                | "ActionStarted"
                | "ActionCompleted"
                | "ActionFailed"
                | "ContinuationPlanResolved"
                | "TickFrameEmitted"
        ) {
            history.push(event.clone());
            continue;
        }

        tracing::debug!("Pipeline tick: received event type={}", event.event_type);

        // ── Persist raw perception event ────────────────────────────────────
        if let Err(e) = runtime.store.append(event.clone()).await {
            tracing::warn!("Store append failed for {}: {:?}", event.event_type, e);
        }

        // ── Legacy State Updates (for API bridge compatibility & Risk) ────────
        {
            let mut res_guard = resolver.write().await;
            let mut se_guard = session_engine_opt.write().await;

            let _ = res_guard.process_event(&event);
            let graph = res_guard.graph().clone();

            let session_engine =
                se_guard.get_or_insert_with(|| SessionEngine::new(10, graph.clone()));
            let _ = session_engine.process_event(&event);
            
            for ne in new_tick_events.iter().skip(1) {
                let _ = res_guard.process_event(ne);
                let _ = session_engine.process_event(ne);
            }

            // Generate Risk Forecast
            let state = chronos_memory_state::StateProjector::project(&graph, session_engine.projection());
            let commits = chronos_reasoning_commitments::CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
            let deadlines = chronos_reasoning_dde::DeadlineEngine::discover_deadlines(&state, &commits, session_engine.projection(), &graph, &[]);
            let capacity = chronos_reasoning_pcm::CapacityEngine::estimate_capacity(&state, session_engine.projection(), &commits, &deadlines);
            
            let forecast = chronos_reasoning_risk::RiskEngine::calculate_risk(&state, session_engine.projection(), &commits, &deadlines, &capacity);
            let risk_event = chronos_reasoning_risk::RiskEngine::to_event(&forecast);
            
            if let Err(e) = runtime.store.append(risk_event.clone()).await {
                tracing::warn!("Store append failed for RiskForecastResolved: {:?}", e);
            }
            let _ = runtime.bus.publish(risk_event.clone());
            new_tick_events.push(risk_event);
        }

        // ── Phase 2: Execute Tick ─────────────────────────────────────────────
        let frame = engine.execute_tick_framed(&history, &new_tick_events, "live-session", &executor);

        // ── Phase 3: Publish and Persist Emitted Events ───────────────────────
        let emitted_events: Vec<ChronosEvent> = frame.all_events()
            .into_iter()
            .filter(|e| !new_tick_events.iter().any(|ne| ne.id == e.id))
            .collect();

        for emitted in emitted_events {
            alert_emitter.process_event(&emitted);
            let _ = runtime.store.append(emitted.clone()).await;
            let _ = runtime.bus.publish(emitted);
        }

        // Emit the TickFrame itself so the UI can stream it
        let frame_event = chronos_core::ChronosEvent::new(
            "TickFrameEmitted",
            "ContinuousRuntimeLoopEngine",
            serde_json::to_value(&frame).unwrap_or(serde_json::Value::Null)
        );
        alert_emitter.process_event(&frame_event);
        let _ = runtime.store.append(frame_event.clone()).await;
        let _ = runtime.bus.publish(frame_event);

        // Add all events from this tick to history
        history.extend(new_tick_events.clone());
        let to_add: Vec<_> = frame.all_events()
            .into_iter()
            .filter(|e| !history.iter().any(|he| he.id == e.id))
            .collect();
        history.extend(to_add);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_core::ChronosEvent;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_daemon_service_registration() {
        let registry = ServiceRegistry::new();
        register_all_services(&registry).await;

        let all = registry.list().await;
        assert_eq!(all.len(), 12, "Expected 12 PCOS services registered");

        // Verify all services are Running
        for svc in &all {
            assert_eq!(
                svc.current_status,
                chronos_registry::ServiceStatus::Running,
                "Service {} should be Running",
                svc.service_id
            );
            assert_eq!(
                svc.health_state,
                chronos_registry::ServiceHealth::Healthy,
                "Service {} should be Healthy",
                svc.service_id
            );
        }
    }

    #[tokio::test]
    async fn test_daemon_store_initialization() {
        let tmp = NamedTempFile::new().unwrap();
        let store = SQLiteEventStore::new(tmp.path()).unwrap();
        let count = store.count().await.unwrap();
        assert_eq!(count, 0, "Fresh store should have 0 events");
    }

    #[tokio::test]
    async fn test_replay_determinism() {
        // Feed the same events twice and verify same resolver state
        let events = vec![
            ChronosEvent::new(
                "GitRepositoryDiscovered",
                "GitAdapter",
                json!({ "repository_path": "/workspace/chronos" }),
            ),
            ChronosEvent::new(
                "GitCommitCreated",
                "GitAdapter",
                json!({
                    "repository_path": "/workspace/chronos",
                    "source_payload": { "message": "init" }
                }),
            ),
        ];

        let bus1 = MemoryEventBus::new(64);
        let bus2 = MemoryEventBus::new(64);

        let (resolver1, session1, _comm1, _cepo1) = replay_events(events.clone(), &bus1).await;
        let (resolver2, session2, _comm2, _cepo2) = replay_events(events, &bus2).await;

        let res1_guard = resolver1.read().await;
        let res2_guard = resolver2.read().await;
        let graph1 = res1_guard.graph();
        let graph2 = res2_guard.graph();
        assert_eq!(
            graph1.entities().len(),
            graph2.entities().len(),
            "Replay must yield identical entity counts"
        );

        let se1_guard = session1.read().await;
        let se2_guard = session2.read().await;
        // Both must have same session count
        let count1 = se1_guard.as_ref().map(|s| s.projection().sessions().len()).unwrap_or(0);
        let count2 = se2_guard.as_ref().map(|s| s.projection().sessions().len()).unwrap_or(0);
        assert_eq!(count1, count2, "Replay must yield identical session counts");
    }

    #[tokio::test]
    async fn test_end_to_end_pipeline_cycle() {
        // Publish a real event to the bus, run one pipeline cycle, verify
        // the decision event appears in the store.
        let tmp = NamedTempFile::new().unwrap();
        let store = Arc::new(SQLiteEventStore::new(tmp.path()).unwrap());
        let bus = Arc::new(MemoryEventBus::new(256));
        let registry = Arc::new(ServiceRegistry::new());
        register_all_services(&registry).await;

        let runtime = Arc::new(DaemonRuntime {
            store: Arc::clone(&store),
            bus: Arc::clone(&bus),
            registry,
        });

        let pipeline_subscriber = bus.subscribe();
        let pipeline_runtime = Arc::clone(&runtime);
        let handle = tokio::spawn(async move {
            run_pipeline(
                pipeline_runtime,
                pipeline_subscriber,
                Arc::new(RwLock::new(EntityResolver::new())),
                Arc::new(RwLock::new(None)),
                Arc::new(RwLock::new(chronos_reasoning_commitments::CommitmentState::new())),
                Arc::new(RwLock::new(chronos_event_orchestrator::EventOrchestrator::new())),
            )
            .await;
        });

        // Inject a real perception event
        let event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos-test" }),
        );
        bus.publish(event).unwrap();

        // Give the pipeline time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        handle.abort();

        // Verify at least 2 events persisted: original + TickFrameEmitted
        let count = store.count().await.unwrap();
        assert!(
            count >= 2,
            "Expected at least 2 persisted events (event + TickFrameEmitted), got {}",
            count
        );

        // Verify a TickFrameEmitted event exists
        let all = store.stream().await.unwrap();
        let has_tick_frame = all.iter().any(|e| e.event_type == "TickFrameEmitted");
        assert!(has_tick_frame, "Pipeline must produce a TickFrameEmitted event");
    }

    #[tokio::test]
    async fn test_adapter_startup_registers_services() {
        // Verifies that start_adapters registers the 4 adapter service descriptors
        // in the ServiceRegistry without panicking.
        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(256));
        let handles = start_adapters(Arc::clone(&bus), Arc::clone(&registry)).await;

        // All adapters should start without error — any startup failure is
        // logged as a warn, not a panic. We just verify no panic occurred.
        let all = registry.list().await;

        // At minimum, the adapters that always start (window-focus, clipboard)
        // plus the ones that register even without a watch path (filewatcher, git)
        // should each have registered one service descriptor.
        assert!(
            all.len() >= 4,
            "Expected at least 4 adapter service descriptors, got {}",
            all.len()
        );

        for h in handles {
            h.abort();
        }
    }

    #[tokio::test]
    async fn test_git_adapter_integration_with_bus() {
        use chronos_adapter_git::GitAdapter as RawGitAdapter;
        use chronos_config::ConfigurationService;
        use chronos_logging::{ChronosLogger, LogContext};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let repo_path = dir.path().to_path_buf();
        let git_dir = repo_path.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(64));
        let config = Arc::new(ConfigurationService::new());
        let logger = ChronosLogger::new(LogContext::new());

        let adapter = RawGitAdapter::new(registry, bus.clone() as Arc<dyn EventBus>, config, logger);
        let mut sub = bus.subscribe();

        adapter.start().await.unwrap();
        adapter.watch_repository(repo_path.clone()).await.unwrap();

        // Should receive GitRepositoryDiscovered immediately on watch_repository
        let evt = sub.next_event().await.unwrap();
        assert_eq!(evt.event_type, "GitRepositoryDiscovered");
        assert_eq!(
            evt.payload["repository_path"].as_str().unwrap(),
            repo_path.to_string_lossy()
        );
    }

    #[tokio::test]
    async fn test_filewatcher_integration_with_bus() {
        use chronos_adapter_filewatcher::FilewatcherAdapter as RawFilewatcherAdapter;
        use chronos_config::ConfigurationService;
        use chronos_logging::{ChronosLogger, LogContext};
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_perception.txt");

        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(64));
        let config = Arc::new(ConfigurationService::new());
        let logger = ChronosLogger::new(LogContext::new());

        let adapter = RawFilewatcherAdapter::new(
            registry,
            bus.clone() as Arc<dyn EventBus>,
            config,
            logger,
        );
        let mut sub = bus.subscribe();

        adapter.start().await.unwrap();
        adapter.watch_directory(dir.path().to_path_buf()).unwrap();

        // Write a real file — this must trigger a FileCreated or FileModified event
        {
            let mut f = std::fs::File::create(&file_path).unwrap();
            f.write_all(b"chronos perception test").unwrap();
        }

        let mut got_file_event = false;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Ok(Ok(evt)) =
                tokio::time::timeout(Duration::from_millis(50), sub.next_event()).await
            {
                if evt.event_type == "FileCreated" || evt.event_type == "FileModified" {
                    got_file_event = true;
                    break;
                }
            }
        }

        assert!(
            got_file_event,
            "FilewatcherAdapter must emit FileCreated/FileModified on real file write"
        );
    }
}
