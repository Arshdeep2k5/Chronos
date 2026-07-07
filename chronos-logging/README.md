# chronos-logging

Unified structured observability layer for the Chronos Personal Context Operating System (PCOS).

## Specification

### Consumes
- **Subsystem Contexts**: Subsystems pass logs enriched with metadata keys (e.g. `subsystem_id`, `correlation_id`, `session_id`, `event_id`, `project_id`, `commitment_id`).
- **Log Levels**: Standard levels `TRACE`, `DEBUG`, `INFO`, `WARN`, and `ERROR`.
- **Custom Payload Metadata**: Any arbitrary serializable key-value data attached to `LogContext` custom fields.

### Produces
- **Structured JSON Streams**: Flat JSON outputs suitable for machine parsing, search indexing, or downstream event ingestion.
- **Human-Readable Terminal Feeds**: Color-coded, compact log formats for local debugging and CLI output.

### Capabilities
- **Structured Fields**: Natively tracks critical PCOS entities (events, sessions, projects, commitments) in a structured manner.
- **Context Chaining/Derivation**: Support deriving specialized child loggers with merged parent-child context parameters.
- **Global Initialization**: Thread-safe configuration of stdout layers using `tracing-subscriber` and environment filters.

### Dependencies
- `tracing` and `tracing-subscriber`: Under the hood, we leverage Rust's canonical tracing ecosystem.
- `serde` and `serde_json`: Used to manage serialized JSON contexts.
- `chrono`: Handles high-precision ISO timestamps for events.

### Failure Modes
- **Double-Initialization**: Attempting to initialize the global subscriber context multiple times (handled gracefully by returning a `SubscriberInitExt` error/result).
- **Serialization Failure**: If custom fields contain non-serializable objects (handled by defaulting field representation to empty JSON values).
