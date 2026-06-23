# API & IPC Contract Document: Chronos Pilot (v1.0)

This contract specifies the structured JSON schemas exchanged over secure localhost TCP loopback between the Browser, IDE, Frontend, and Core Daemon.

## 1. Network Handshake Verification
All WebSocket connections must pass authorization headers containing a dynamic API token:
* **Token File Path**: `~/.config/chronos/handshake.json`
* **JSON Schema**: `{"auth_token": "CRYPTOGRAPHIC_HEX_TOKEN"}`
* Any telemetry connection missing this signature is instantly aborted.

## 2. API Contract Specifications

### 2.1 Browser Extension Telemetry → Daemon (WS)
* **Endpoint**: `ws://localhost:48120/telemetry/browser`
* **Message Type**: `TAB_FOCUS`

```json
{
  "type": "TAB_FOCUS",
  "auth_token": "9a3f2b1d0c...",
  "payload": {
    "url": "https://docs.stripe.com/webhooks/signatures",
    "title": "Webhook Signatures | Stripe Documentation",
    "domain": "docs.stripe.com",
    "visit_started_at": "2026-06-15T10:14:00Z"
  }
}
```

* **Message Type**: `SEARCH_QUERY`

```json
{
  "type": "SEARCH_QUERY",
  "auth_token": "9a3f2b1d0c...",
  "payload": {
    "url": "https://www.google.com/search?q=stripe+webhook+verification+failure",
    "query_text": "stripe webhook verification failure",
    "created_at": "2026-06-15T10:15:20Z"
  }
}
```

### 2.2 VSCode Connector Telemetry ◄► Daemon (WS)
* **Endpoint**: `ws://localhost:48120/telemetry/ide`
* **Message Type**: `WORKSPACE_TELEMETRY` (Inbound to Daemon)

```json
{
  "type": "WORKSPACE_TELEMETRY",
  "auth_token": "9a3f2b1d0c...",
  "payload": {
    "active_file_path": "/Users/developer/billing-system/billing/stripe_webhook.ts",
    "cursor_line": 42,
    "cursor_column": 12,
    "open_tabs": [
      "/Users/developer/billing-system/package.json",
      "/Users/developer/billing-system/billing/stripe_webhook.ts"
    ],
    "timestamp": "2026-06-15T10:30:00Z"
  }
}
```

* **Message Type**: `WORKSPACE_RESTORE` (Outbound to VSCode)

```json
{
  "type": "WORKSPACE_RESTORE",
  "payload": {
    "directory_path": "/Users/developer/billing-system",
    "active_file_path": "/Users/developer/billing-system/billing/stripe_webhook.ts",
    "cursor_line": 42,
    "cursor_column": 12,
    "open_tabs": [
      "/Users/developer/billing-system/package.json",
      "/Users/developer/billing-system/billing/stripe_webhook.ts"
    ]
  }
}
```

### 2.3 Core Daemon ──► SolidJS HUD Frontend (Tauri IPC Bridge)
* **Event**: `PROJECT_RISK_UPDATE`

```json
{
  "project_id": 1,
  "commitment_id": 12,
  "health_status": "RED",
  "risk_score": 0.84,
  "timeline_completion_probability": 0.41,
  "predicted_failure_date": "2026-07-12",
  "consequence_simulation": {
    "completion_probability_now": 0.41,
    "completion_probability_tomorrow": 0.28,
    "marginal_loss_24h": 0.13
  }
}
```

* **Event**: `MOMENTUM_LOSS_TRIGGER`

```json
{
  "project_id": 1,
  "last_active": "2026-06-12T10:30:00Z",
  "diagnostics": {
    "likely_stopping_point": "Webhook signature validation failure",
    "last_open_file": "billing/stripe_webhook.ts",
    "last_search_query": "stripe webhook local signature test failure",
    "last_checkpoint_blocker": "Stuck trying to mock Stripe signatures locally."
  }
}
```
