# Privacy & Security Whitepaper: Chronos Pilot (v1.0)

Chronos Pilot is engineered with privacy as a structural design constraint. Telemetry trackers and analysis systems execute entirely under user sovereignty.

## 1. Local-First Guarantees
* **Data Isolation**: All parsed text, search inputs, code filenames, and context files reside strictly in a localized database (`~/.config/chronos/chronos.db`).
* **Zero Cloud Pipelines**: No remote analytics engines, cloud telemetry systems, or cloud databases are initialized.
* **Offline Functionality**: The system requires no active network connection to process database updates, compute failure risk curves, run search graph traversals, or execute workspace restoration loops.

## 2. Data Collection Boundaries

### 2.1 Collected Information
* **Workspaces**: Open file paths, code directory trees, line/column cursor positions, and focused editor durations.
* **Browser Activity**: Active page URL, active page header title, search queries, active focus seconds, and visit start/end timestamps.
* **Documents**: Explicit text contents of ingested PDFs, DOCX, and Markdown documents inside watched directories.

### 2.2 Data Expressly Prohibited (Exclusion Boundaries)
To ensure complete system safety, Chronos Pilot implements strict collection boundaries:
* **Secure Inputs**: The system is physically blind to password text entry elements, banking domains, secure parameters, credit card numbers, and secure sessions.
* **Private Communications**: Telemetry connectors do not capture personal messages, chat histories, active screen captures, or key logs.
* **Session Cookies**: No session cookies, tokens, or credential handshakes are stored or processed.

## 3. Connection Authorization (Loopback Protection)
Connections between external connectors (VSCode extension, browser extension) and the Core Daemon execute strictly over local loopback interfaces (`localhost`):
* Connections attempting traversal from external non-local IPs are blocked at the socket binding layer.
* Connection transactions require a dynamic cryptographic token generated at runtime and stored locally inside `handshake.json`.

## 4. User Data Erasure & Control
Users maintain absolute control over stored telemetry records:
* **Inspection Dashboard**: SolidJS HUD contains an visual dashboard to query history tables.
* **Itemized Deletion**: Users can search, filter, and purge individual URLs, files, or search queries from database history.
* **Total System Reset**: Clicking "Purge System Data" runs clean database drops, removing the SQL tables on disk and restoring the system to factory baseline conditions.
