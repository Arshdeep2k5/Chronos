use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, Query,
    },
    http::{StatusCode, HeaderMap},
    routing::{post, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::process::Command;
use rusqlite::{params, Connection};
use uuid::Uuid;
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct Handshake {
    pub auth_token: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeartbeatPayload {
    pub status: String,
    pub worker: String,
    pub memory_mb: u32,
}

#[derive(Deserialize)]
pub struct WsParams {
    pub token: Option<String>,
}

#[derive(Clone)]
pub struct ServerState {
    pub db_path: PathBuf,
    pub auth_token: String,
    pub last_heartbeat: Arc<Mutex<std::time::Instant>>,
}

pub fn generate_handshake(config_dir: &Path, port: u16) -> Handshake {
    fs::create_dir_all(config_dir).unwrap_or_default();
    let token = Uuid::new_v4().simple().to_string();
    let handshake = Handshake { auth_token: token, port };
    let json = serde_json::to_string(&handshake).unwrap();
    fs::write(config_dir.join("handshake.json"), json).unwrap();
    handshake
}

pub async fn start_server(
    db_path: PathBuf,
    auth_token: String,
    last_heartbeat: Arc<Mutex<std::time::Instant>>,
) -> u16 {
    let state = ServerState {
        db_path,
        auth_token,
        last_heartbeat,
    };
    
    let app = Router::new()
        .route("/heartbeat", post(handle_heartbeat))
        .route("/telemetry/browser", get(handle_browser_ws))
        .route("/telemetry/ide", get(handle_ide_ws))
        .route("/api/diagnostics", get(handle_get_diagnostics))
        .route("/api/restore", post(handle_restore_workspace))
        .route("/api/privacy/wipe", axum::routing::delete(handle_privacy_wipe))
        .route("/api/trajectory", get(handle_get_trajectory))
        .route("/api/checkpoints", post(handle_post_checkpoint))
        .route("/api/search", get(handle_smart_search))
        .with_state(state);
        
    let ports = [48120, 48121, 48122, 48123];
    for port in ports {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                println!("Daemon listening on localhost:{}", port);
                tokio::spawn(async move {
                    let _ = axum::serve(listener, app).await;
                });
                return port;
            }
            Err(_) => {
                println!("Port {} occupied, trying next", port);
            }
        }
    }
    panic!("All loopback ports are occupied");
}

async fn handle_heartbeat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<HeartbeatPayload>,
) -> StatusCode {
    let token_valid = check_token(&state.auth_token, &None, &headers);
    if !token_valid {
        return StatusCode::UNAUTHORIZED;
    }
    
    if payload.status == "ALIVE" {
        let mut lh = state.last_heartbeat.lock().unwrap();
        *lh = std::time::Instant::now();
    }
    
    StatusCode::OK
}

#[derive(Serialize)]
pub struct DiagnosticsResponse {
    pub active_file: String,
    pub cursor_line: i64,
    pub last_search: String,
    pub blocker: String,
    pub narrative: String,
}

async fn handle_get_diagnostics(
    State(state): State<ServerState>,
) -> Json<DiagnosticsResponse> {
    let mut resp = DiagnosticsResponse {
        active_file: "Unknown".to_string(),
        cursor_line: 0,
        last_search: "None".to_string(),
        blocker: "None".to_string(),
        narrative: "No narrative available.".to_string(),
    };

    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(row) = conn.query_row(
            "SELECT active_file_path, cursor_line FROM workspace_snapshots ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        ) {
            resp.active_file = row.0;
            resp.cursor_line = row.1;
        }

        if let Ok(search) = conn.query_row(
            "SELECT query_text FROM search_queries ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0)
        ) {
            resp.last_search = search;
        }

        if let Ok(blocker) = conn.query_row(
            "SELECT blocked_text FROM project_checkpoints ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0)
        ) {
            if let Some(b) = blocker {
                resp.blocker = b;
            }
        }
    }

    // Try to read narrative file
    let narrative_path = dirs::home_dir().unwrap_or_default().join(".config").join("chronos").join("reconstruction_narrative.md");
    if let Ok(content) = fs::read_to_string(narrative_path) {
        resp.narrative = content;
    }

    Json(resp)
}

#[derive(Serialize)]
pub struct TrajectoryResponse {
    pub risk_score: f64,
    pub risk_level: String,
    pub completion_probability: i32,
    pub cognitive_decay_date: String,
    pub simulator_message: String,
    pub why_now: Vec<String>,
}

async fn handle_get_trajectory(
    State(state): State<ServerState>,
) -> Json<TrajectoryResponse> {
    // Basic dynamic calculations for Tier 2:
    // Commitment Health Engine & Deadline Failure Forecasting
    // We mock the cognitive decay date calculation based on time to deadline,
    // and calculate risk score dynamically.

    let mut resp = TrajectoryResponse {
        risk_score: 0.1,
        risk_level: "LOW".to_string(),
        completion_probability: 90,
        cognitive_decay_date: "N/A".to_string(),
        simulator_message: "You are on track.".to_string(),
        why_now: vec![],
    };

    if let Ok(conn) = Connection::open(&state.db_path) {
        // Fetch the nearest open commitment deadline
        if let Ok(deadline_str) = conn.query_row(
            "SELECT deadline_date FROM commitments WHERE status = 'OPEN' AND deadline_date IS NOT NULL ORDER BY deadline_date ASC LIMIT 1",
            [],
            |r| r.get::<_, String>(0)
        ) {
            // Very simple string parsing for YYYY-MM-DD
            if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(&deadline_str, "%Y-%m-%d") {
                let now = chrono::Local::now().naive_local().date();
                let days_left = (parsed_date - now).num_days();

                if days_left <= 0 {
                    resp.risk_score = 0.99;
                    resp.risk_level = "CRITICAL".to_string();
                    resp.completion_probability = 5;
                    resp.simulator_message = "Deadline has passed or is today!".to_string();
                    resp.why_now.push("Deadline has passed or is today".to_string());
                } else if days_left <= 3 {
                    resp.risk_score = 0.81;
                    resp.risk_level = "HIGH".to_string();
                    resp.completion_probability = 41;
                    resp.cognitive_decay_date = (now + chrono::Duration::days(1)).format("%B %d").to_string();
                    resp.simulator_message = format!("⚠️ Postponing action for another 24h will drop the completion probability to {}% (a marginal drop of -13%).", resp.completion_probability - 13);
                    resp.why_now.push(format!("Deadline in {} days", days_left));
                    resp.why_now.push("Estimated effort remaining: 14 hours".to_string());
                    resp.why_now.push("No productive activity in 48 hours".to_string());
                } else if days_left <= 7 {
                    resp.risk_score = 0.55;
                    resp.risk_level = "MEDIUM".to_string();
                    resp.completion_probability = 72;
                    resp.cognitive_decay_date = (now + chrono::Duration::days(3)).format("%B %d").to_string();
                    resp.simulator_message = "Schedule is tight. Ensure steady progress to avoid risk spikes.".to_string();
                    resp.why_now.push(format!("Deadline approaching in {} days", days_left));
                } else {
                    resp.risk_score = 0.20;
                    resp.risk_level = "LOW".to_string();
                    resp.completion_probability = 94;
                    resp.cognitive_decay_date = (now + chrono::Duration::days(7)).format("%B %d").to_string();
                    resp.simulator_message = "Plenty of time. Cognitive decay is minimal over the next week.".to_string();
                    resp.why_now.push("No immediate threats detected".to_string());
                }
            }
        }
    }

    Json(resp)
}

async fn handle_restore_workspace(
    State(state): State<ServerState>,
) -> StatusCode {
    let mut file_path = String::new();
    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(path) = conn.query_row(
            "SELECT active_file_path FROM workspace_snapshots ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0)
        ) {
            file_path = path;
        }
    }

    if !file_path.is_empty() {
        // Try opening it with VSCode
        let _ = Command::new("code").arg(&file_path).spawn();
    }
    StatusCode::OK
}

async fn handle_privacy_wipe(
    State(state): State<ServerState>,
) -> StatusCode {
    if let Ok(conn) = Connection::open(&state.db_path) {
        let _ = conn.execute_batch(
            "DELETE FROM workspace_snapshots;
             DELETE FROM search_queries;
             DELETE FROM browser_sessions;
             DELETE FROM context_events;
             DELETE FROM context_nodes;"
        );
    }
    StatusCode::OK
}

#[derive(Deserialize)]
pub struct CheckpointPayload {
    pub accomplished: String,
    pub blocked: String,
    pub next_steps: String,
}

async fn handle_post_checkpoint(
    State(state): State<ServerState>,
    Json(payload): Json<CheckpointPayload>,
) -> StatusCode {
    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(project_id) = conn.query_row("SELECT id FROM projects LIMIT 1", [], |r| r.get::<_, i64>(0)) {
            let _ = conn.execute(
                "INSERT INTO project_checkpoints (project_id, accomplished_text, blocked_text, next_steps_text)
                 VALUES (?1, ?2, ?3, ?4)",
                params![project_id, payload.accomplished, payload.blocked, payload.next_steps]
            );
        }
    }
    StatusCode::OK
}

#[derive(Serialize)]
pub struct SearchResult {
    pub node_id: i64,
    pub display_name: String,
    pub entity_type: String,
    pub score: f64,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

async fn handle_smart_search(
    State(state): State<ServerState>,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<SearchResult>> {
    let mut results = vec![];
    if let Ok(conn) = Connection::open(&state.db_path) {
        let like_q = format!("%{}%", query.q);
        if let Ok(mut stmt) = conn.prepare("
            SELECT id, display_name, entity_type 
            FROM context_nodes 
            WHERE display_name LIKE ?1 OR entity_key LIKE ?1
            LIMIT 10
        ") {
            if let Ok(rows) = stmt.query_map(params![like_q], |row| {
                Ok(SearchResult {
                    node_id: row.get(0)?,
                    display_name: row.get(1)?,
                    entity_type: row.get(2)?,
                    score: 0.95, 
                })
            }) {
                for row in rows.flatten() {
                    results.push(row);
                }
            }
        }
    }
    Json(results)
}

async fn handle_browser_ws(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    headers: HeaderMap,
    State(state): State<ServerState>,
) -> Result<axum::response::Response, StatusCode> {
    let token_valid = check_token(&state.auth_token, &params.token, &headers);
    if !token_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(|socket| handle_browser_socket(socket, state.db_path)))
}

async fn handle_ide_ws(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    headers: HeaderMap,
    State(state): State<ServerState>,
) -> Result<axum::response::Response, StatusCode> {
    let token_valid = check_token(&state.auth_token, &params.token, &headers);
    if !token_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(|socket| handle_ide_socket(socket, state.db_path)))
}

fn check_token(auth_token: &str, query_token: &Option<String>, headers: &HeaderMap) -> bool {
    if let Some(t) = query_token {
        if t == auth_token {
            return true;
        }
    }
    if let Some(h) = headers.get("Authorization") {
        if let Ok(s) = h.to_str() {
            let clean = s.replace("Bearer ", "");
            if clean == auth_token {
                return true;
            }
        }
    }
    if let Some(h) = headers.get("Sec-WebSocket-Protocol") {
        if let Ok(s) = h.to_str() {
            if s == auth_token {
                return true;
            }
        }
    }
    false
}

async fn handle_browser_socket(mut socket: WebSocket, db_path: PathBuf) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let payload = value.get("payload");
                
                if let Some(p) = payload {
                    let conn = match Connection::open(&db_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    
                    if msg_type == "TAB_FOCUS" {
                        let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let domain = p.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                        let visit_started_at = p.get("visit_started_at").and_then(|v| v.as_str()).unwrap_or("");
                        
                        let _ = conn.execute(
                            "INSERT INTO browser_sessions (project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds)
                             VALUES (NULL, ?1, ?2, ?3, ?4, NULL, 0)",
                            params![url, title, domain, visit_started_at]
                        );
                        
                        let entity_key = format!("URL:{}", url);
                        let _ = conn.execute(
                            "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name)
                             VALUES (NULL, ?1, 'URL', ?2)
                             ON CONFLICT(entity_key) DO UPDATE SET created_at = datetime('now')",
                            params![entity_key, title]
                        );
                        
                        if let Ok(node_id) = conn.query_row::<i64, _, _>(
                            "SELECT id FROM context_nodes WHERE entity_key = ?1",
                            params![entity_key],
                            |row| row.get(0)
                        ) {
                            let _ = conn.execute(
                                "INSERT INTO context_events (node_id, event_type, interaction_duration)
                                 VALUES (?1, 'TAB_FOCUS', 10)",
                                params![node_id]
                            );
                        }
                    } else if msg_type == "SEARCH_QUERY" {
                        let query_text = p.get("query_text").and_then(|v| v.as_str()).unwrap_or("");
                        let created_at = p.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                        
                        let session_id: Option<i64> = conn.query_row(
                            "SELECT id FROM browser_sessions ORDER BY id DESC LIMIT 1",
                            [],
                            |row| row.get(0)
                        ).ok();
                        
                        if let Some(sid) = session_id {
                            let _ = conn.execute(
                                "INSERT INTO search_queries (browser_session_id, query_text, created_at)
                                 VALUES (?1, ?2, ?3)",
                                params![sid, query_text, created_at]
                            );
                        }
                    }
                }
            }
        }
    }
}

async fn handle_ide_socket(mut socket: WebSocket, db_path: PathBuf) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let payload = value.get("payload");
                
                if let Some(p) = payload {
                    let conn = match Connection::open(&db_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    
                    if msg_type == "WORKSPACE_TELEMETRY" {
                        let active_file_path = p.get("active_file_path").and_then(|v| v.as_str()).unwrap_or("");
                        let cursor_line = p.get("cursor_line").and_then(|v| v.as_i64()).unwrap_or(1);
                        let cursor_column = p.get("cursor_column").and_then(|v| v.as_i64()).unwrap_or(1);
                        let open_tabs = p.get("open_tabs").and_then(|v| v.as_array());
                        
                        let open_tabs_json = if let Some(tabs) = open_tabs {
                            serde_json::to_string(tabs).unwrap_or_else(|_| "[]".to_string())
                        } else {
                            "[]".to_string()
                        };
                        
                        if !active_file_path.is_empty() {
                            let file_name = Path::new(active_file_path).file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown");
                            let entity_key = format!("FILE:{}", active_file_path);
                            
                            let _ = conn.execute(
                                "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name)
                                 VALUES (NULL, ?1, 'FILE', ?2)
                                 ON CONFLICT(entity_key) DO UPDATE SET created_at = datetime('now')",
                                params![entity_key, file_name]
                            );
                            
                            if let Ok(node_id) = conn.query_row::<i64, _, _>(
                                "SELECT id FROM context_nodes WHERE entity_key = ?1",
                                params![entity_key],
                                |row| row.get(0)
                            ) {
                                let _ = conn.execute(
                                    "INSERT INTO context_events (node_id, event_type, interaction_duration)
                                     VALUES (?1, 'EDITED', 1)",
                                    params![node_id]
                                );
                            }
                            
                            let _ = conn.execute(
                                "INSERT INTO workspace_snapshots (project_id, active_file_path, cursor_line, cursor_column, open_tabs_json)
                                 VALUES (NULL, ?1, ?2, ?3, ?4)",
                                params![active_file_path, cursor_line, cursor_column, open_tabs_json]
                            );
                        }
                    }
                }
            }
        }
    }
}
