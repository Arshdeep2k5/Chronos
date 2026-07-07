use criterion::{criterion_group, criterion_main, Criterion};
use chronos_core::ChronosEvent;
use chronos_runtime_loop::{ContinuousRuntimeLoopEngine, RuntimeMode};
use chronos_execution_orchestration::DefaultMockExecutor;

fn bench_execute_tick(c: &mut Criterion) {
    let engine = ContinuousRuntimeLoopEngine::new(RuntimeMode::Live);
    let executor = DefaultMockExecutor;
    
    // Setup mock events
    let event = ChronosEvent::new(
        "VSCodeActiveFileChanged",
        "VSCodeActiveFileObserver",
        serde_json::json!({ "file_path": "src/main.rs" })
    );
    let history = vec![event.clone()];
    let new_events = vec![event];
    
    c.bench_function("execute_tick_framed", |b| {
        b.iter(|| {
            let _ = engine.execute_tick_framed(&history, &new_events, "bench-session", &executor);
        })
    });
}

criterion_group!(benches, bench_execute_tick);
criterion_main!(benches);
