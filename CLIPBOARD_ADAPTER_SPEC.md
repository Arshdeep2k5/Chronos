# Clipboard Adapter Specification

The Clipboard Adapter is a Layer 1 (Perception) subsystem that observes native OS clipboard events, extracts metadata on copy transactions, deduplicates consecutive duplicate payloads, and publishes standardized context events to the `Cognitive Bus`.

---

## 1. Specification

### Consumes
*   **OS Clipboard Data**: Native Windows clipboard APIs polling at regular intervals.

### Produces
*   `ClipboardChanged`
*   `ClipboardTextCopied`
*   `ClipboardFileCopied`
*   `ClipboardUriCopied`
*   `ClipboardImageCopied`

---

## 2. Event Payload Schema

Every published clipboard event payload contains:
```json
{
  "timestamp": "2026-06-28T12:00:00Z",
  "content_type": "text",
  "content_hash": "2f42a59a...",
  "source_application": "Code.exe",
  "content_size": 256,
  "file_count": 0
}
```

For file copies (`CF_HDROP`), the payload looks like:
```json
{
  "timestamp": "2026-06-28T12:00:00Z",
  "content_type": "files",
  "content_hash": "19b4d82f...",
  "source_application": "explorer.exe",
  "content_size": 4096,
  "file_count": 3
}
```

---

## 3. Boundary & Debouncing Logic
*   **Duplicate Suppression**: Identical clipboard content is ignored on successive polls.
*   **Empty Clipboard Suppression**: Empty clipboard states or unsupported clip formats are ignored.
*   **Replay Safety**: Volatile clipboard transitions are published as immutable `ChronosEvent`s, which can be stored in the `EventStore` and replayed cleanly. No reasoning, forecasting, or AI inference is applied.
