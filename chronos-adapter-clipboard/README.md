# chronos-adapter-clipboard

**Layer 1 — Perception Adapter**

Observes OS clipboard transitions, classifies content type (text, URI, files, image), deduplicates identical payloads, and publishes factual `ChronosEvent`s onto the Cognitive Bus.

## Architecture

```
OS Clipboard
    ↓
ClipboardObserver (polls every 500ms)
    ↓
Duplicate hash check → suppress if same as last
    ↓
ClipboardEventNormalizer (classify + build payload)
    ↓
EventBus.publish()
```

## Published Events

| Event | Trigger |
|-------|---------|
| `ClipboardChanged` | Any new clipboard content |
| `ClipboardTextCopied` | Plain text copied |
| `ClipboardUriCopied` | URL/URI copied (http://, https://, file://) |
| `ClipboardFileCopied` | Files copied (CF_HDROP) |
| `ClipboardImageCopied` | Image copied (CF_DIB / CF_DIBV5) |

## Payload Fields

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | ISO8601 | When the clipboard change was detected |
| `content_type` | string | `text`, `uri`, `files`, or `image` |
| `content_hash` | hex string | Hash of content for deduplication |
| `source_application` | string | Name of the process that wrote to clipboard |
| `content_size` | u64 | Size in bytes of the global memory handle |
| `file_count` | u32 | Number of files (for `files` type, else 0) |

## Capabilities Registered

- `ObserveClipboard`

## Dependencies

- `chronos-core` — Event schemas
- `chronos-bus` — Publish to Cognitive Bus
- `chronos-registry` — Register `ObserveClipboard` capability
- `chronos-logging` — Structured logging
- `windows-sys` — Win32 clipboard API bindings

## Platform Notes

The Windows implementation uses:
- `OpenClipboard` / `CloseClipboard` — to safely access clipboard state
- `IsClipboardFormatAvailable` — to detect content type
- `GetClipboardData` + `GlobalLock`/`GlobalUnlock` — to read text
- `DragQueryFileW` — to enumerate file paths from `CF_HDROP`
- `GetClipboardOwner` → `GetWindowThreadProcessId` → `GetModuleFileNameExW` — to identify the source application

The `#[cfg(not(target_os = "windows"))]` stub provides deterministic mock data for cross-platform testing.
