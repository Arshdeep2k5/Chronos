# Window Focus Adapter Specification

The Window Focus Adapter is a Layer 1 (Perception) subsystem. Its sole responsibility is to capture the active foreground window transitions of the OS, normalize this metadata, and publish corresponding context events onto the `Cognitive Bus`.

---

## 1. Specification

### Consumes
*   **OS Foreground Window Notifications**: Native API calls (on Windows, polling or registering hooks for active handle transitions).

### Produces
*   `WindowFocusChanged`
*   `ApplicationActivated`
*   `ApplicationDeactivated`

---

## 2. Event Payload Schema

Every published event payload contains:
```json
{
  "window_title": "D:\\Chronos_Hackathon\\src\\lib.rs - VS Code",
  "process_name": "Code.exe",
  "process_id": 1234,
  "executable_path": "C:\\Program Files\\Microsoft VS Code\\Code.exe",
  "focus_started_at": "2026-06-28T12:00:00Z",
  "focus_ended_at": "2026-06-28T12:00:10Z",
  "duration_ms": 10000
}
```

---

## 3. Boundary & Debouncing Logic
*   **Duplicate Suppression**: If the foreground window handle has not changed, focus notifications are suppressed to prevent spam.
*   **Replay Safety**: OS transitions are volatile runtime states. Once published as immutable `ChronosEvent`s, they are recorded permanently in the `EventStore` for deterministic session replay. No reasoning or AI is used.
