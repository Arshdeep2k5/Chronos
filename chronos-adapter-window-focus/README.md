# chronos-adapter-window-focus

The Window Focus Adapter is a Layer 1 (Perception) subsystem. Its sole responsibility is to capture the active foreground window transitions of the OS, normalize this metadata, and publish corresponding context events onto the `Cognitive Bus`.

## Specification

### Consumes
- **OS Foreground Window Notifications**: Native API calls (on Windows, polling active handle transitions).

### Produces
- `WindowFocusChanged`
- `ApplicationActivated`
- `ApplicationDeactivated`

### Capabilities
- **Native OS Observation**: Queries process name, window title, process ID, and executable path metadata.
- **Switch Timeline Tracking**: Traces transitions and publishes focus duration times.
- **Duplicate Suppression**: Debounces notifications if the active window has not changed.

### Dependencies
- `chronos-core`: Event schemas.
- `chronos-bus`: Bus publication interfaces.
- `chronos-registry`: Registration of capabilities.
- `chronos-logging`: System performance metrics logger.
- `windows-sys`: Win32 native foreground window hooks.

### Failure Modes
- **OpenProcess Access Denied**: Falls back gracefully to generic process names when query permissions are restricted.
