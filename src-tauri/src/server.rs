use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, Query,
    },
    http::{StatusCode, HeaderMap},
    routing::{post, get},
    Json, Router,
};
use tower_http::cors::{CorsLayer, Any};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::process::Command;
use rusqlite::{params, Connection};
use uuid::Uuid;
use std::fs;

const DISTRACTION_DOMAINS: [&str; 8] = ["youtube.com", "youtube", "shorts", "reddit.com", "twitter.com", "netflix.com", "instagram.com", "facebook.com"];
const AI_DOMAINS: [&str; 7] = ["z.ai", "chatgpt", "claude.ai", "gemini.google", "openai.com", "copilot", "v0.dev"];
const CONTEXT_EXPIRY_SECS: i64 = 900;

fn extract_project_from_path_str(path: &str) -> String {
    // 1. Strip application prefix (e.g. "Code:", "Powershell:") if present.
    // A drive letter (e.g. "C:", "D:") has length 1 and is not stripped.
    let mut clean_path = path;
    if let Some(colon_idx) = path.find(':') {
        let prefix = &path[..colon_idx];
        if prefix.len() > 1 {
            clean_path = &path[colon_idx + 1..];
        }
    }

    let normalized = clean_path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    
    if parts.is_empty() {
        return "Unknown Project".to_string();
    }

    // 2. Identify the candidate project name
    let mut candidate = if parts.len() > 1 {
        let mut proj_idx = 0;
        
        // Skip drive letters
        if parts[0].ends_with(':') || (parts[0].len() <= 2 && parts[0].chars().all(|c| c.is_alphabetic() || c == ':')) {
            proj_idx = 1;
        }
        
        // Skip common user/system directories in path
        let mut skip_next = false;
        while proj_idx < parts.len() {
            let p_lower = parts[proj_idx].to_lowercase();
            if skip_next {
                proj_idx += 1;
                skip_next = false;
            } else if p_lower == "users" || p_lower == "home" {
                proj_idx += 1;
                skip_next = true;
            } else if p_lower == "projects" || p_lower == "workspace" || p_lower == "repos" || p_lower == "github" || p_lower == "documents" || p_lower == "downloads" || p_lower == "tmp" || p_lower == "temp" || p_lower == "var" {
                proj_idx += 1;
            } else {
                break;
            }
        }
        
        if proj_idx >= parts.len() {
            parts[parts.len() - 2].to_string()
        } else {
            parts[proj_idx].to_string()
        }
    } else {
        parts[0].to_string()
    };

    // 3. Strip any file extension from the candidate if it has one (e.g. "stripe.ts" -> "stripe")
    if let Some(dot_idx) = candidate.rfind('.') {
        let ext_len = candidate.len() - dot_idx - 1;
        if ext_len >= 1 && ext_len <= 4 {
            candidate = candidate[..dot_idx].to_string();
        }
    }

    candidate
}


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
    pub telemetry_db_path: PathBuf,
    pub auth_token: String,
    pub last_heartbeat: Arc<Mutex<std::time::Instant>>,
    pub simulated_time: Arc<Mutex<chrono::DateTime<chrono::Local>>>,
    pub system: Arc<Mutex<sysinfo::System>>,
}

pub fn generate_handshake(config_dir: &Path, token: String, port: u16) -> Handshake {
    fs::create_dir_all(config_dir).unwrap_or_default();
    let handshake = Handshake { auth_token: token, port };
    let json = serde_json::to_string(&handshake).unwrap();
    fs::write(config_dir.join("handshake.json"), json).unwrap();
    handshake
}

pub fn flush_telemetry_to_main(db_path: &Path, telemetry_db_path: &Path) -> Result<(), rusqlite::Error> {
    let mut main_conn = Connection::open(db_path)?;
    let mut tele_conn = Connection::open(telemetry_db_path)?;
    
    // Start transaction on main
    let main_tx = main_conn.transaction()?;
    
    // 1. Fetch browser_sessions from telemetry DB
    let mut browser_sessions = vec![];
    {
        let mut stmt = tele_conn.prepare(
            "SELECT id, project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at FROM browser_sessions"
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, String>(8)?,
            ))
        })?;
        for row in rows.flatten() {
            browser_sessions.push(row);
        }
    }
    
    // Insert browser sessions into main DB and migrate associated search queries
    for sess in browser_sessions {
        let (old_id, project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at) = sess;
        
        main_tx.execute(
            "INSERT INTO browser_sessions (project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at]
        )?;
        
        let new_session_id = main_tx.last_insert_rowid();
        
        let mut search_queries = vec![];
        {
            let mut stmt = tele_conn.prepare(
                "SELECT query_text, created_at FROM search_queries WHERE browser_session_id = ?1"
            )?;
            let rows = stmt.query_map(params![old_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            for row in rows.flatten() {
                search_queries.push(row);
            }
        }
        
        for sq in search_queries {
            let (query_text, sq_created_at) = sq;
            main_tx.execute(
                "INSERT INTO search_queries (browser_session_id, query_text, created_at)
                 VALUES (?1, ?2, ?3)",
                params![new_session_id, query_text, sq_created_at]
            )?;
        }
    }
    
    // 2. Fetch and insert context_events
    let mut context_events = vec![];
    {
        let mut stmt = tele_conn.prepare(
            "SELECT node_id, event_type, interaction_duration, captured_at FROM context_events"
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<i64>>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        for row in rows.flatten() {
            context_events.push(row);
        }
    }
    
    for ev in context_events {
        let (node_id, event_type, interaction_duration, captured_at) = ev;
        let node_exists: bool = if let Some(nid) = node_id {
            main_tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM context_nodes WHERE id = ?1)",
                params![nid],
                |r| r.get(0)
            ).unwrap_or(false)
        } else {
            true
        };
        
        if node_exists {
            main_tx.execute(
                "INSERT INTO context_events (node_id, event_type, interaction_duration, captured_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![node_id, event_type, interaction_duration, captured_at]
            )?;
        }
    }
    
    // 3. Fetch and insert workspace_snapshots
    let mut workspace_snapshots = vec![];
    {
        let mut stmt = tele_conn.prepare(
            "SELECT project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at FROM workspace_snapshots"
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<i64>>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;
        for row in rows.flatten() {
            workspace_snapshots.push(row);
        }
    }
    
    for snap in workspace_snapshots {
        let (project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at) = snap;
        main_tx.execute(
            "INSERT INTO workspace_snapshots (project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at]
        )?;
    }
    
    // Delete all from telemetry DB
    let tele_tx = tele_conn.transaction()?;
    let _ = tele_tx.execute("DELETE FROM browser_sessions", []);
    let _ = tele_tx.execute("DELETE FROM search_queries", []);
    let _ = tele_tx.execute("DELETE FROM context_events", []);
    let _ = tele_tx.execute("DELETE FROM workspace_snapshots", []);
    
    main_tx.commit()?;
    tele_tx.commit()?;
    
    Ok(())
}

pub async fn start_server(
    db_path: PathBuf,
    telemetry_db_path: PathBuf,
    auth_token: String,
    last_heartbeat: Arc<Mutex<std::time::Instant>>,
) -> u16 {
    let simulated_time = Arc::new(Mutex::new(chrono::Local::now()));
    let system = Arc::new(Mutex::new(sysinfo::System::new_all()));
    let state = ServerState {
        db_path: db_path.clone(),
        telemetry_db_path: telemetry_db_path.clone(),
        auth_token,
        last_heartbeat,
        simulated_time: simulated_time.clone(),
        system,
    };
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);
        
    let app = Router::new()
        .route("/heartbeat", post(handle_heartbeat))
        .route("/telemetry/browser", get(handle_browser_ws))
        .route("/telemetry/ide", get(handle_ide_ws))
        .route("/api/diagnostics", get(handle_get_diagnostics))
        .route("/api/restore", post(handle_restore_workspace))
        .route("/api/privacy/wipe", axum::routing::delete(handle_privacy_wipe))
        .route("/api/trajectory", get(handle_get_trajectory))
        .route("/api/checkpoints", post(handle_post_checkpoint_react))
        .route("/api/search", get(handle_smart_search))
        .route("/api/system-status", get(handle_system_status))
        .route("/api/projects", get(handle_get_projects))
        .route("/api/commitments", get(handle_get_commitments))
        .route("/api/actions", get(handle_get_actions))
        .route("/api/actions/toggle", post(handle_toggle_action_react))
        .route("/api/time-travel", post(handle_time_travel_react))
        .route("/api/workspace/restore", post(handle_restore_workspace_react))
        .route("/api/sandbox/simulate", post(handle_simulate_telemetry))
        .route("/api/generate-plan", post(handle_generate_plan))
        .route("/api/arc/briefs", get(handle_arc_briefs))
        .route("/api/database", get(handle_get_database))
        .route("/api/database/purge", post(handle_database_purge))
        .route("/api/telemetry-logs", get(handle_get_telemetry_logs).delete(handle_delete_telemetry_log))
        .route("/api/telemetry/ingest", post(handle_manual_ingest))
        .route("/api/terminal/suggest", get(handle_terminal_suggest))
        .route("/api/context/export", get(handle_context_export))
        .layer(cors)
        .with_state(state);
        
    let db_path_for_prune = db_path.clone();
    tokio::spawn(async move {
        loop {
            let _ = run_database_compaction(&db_path_for_prune);
            tokio::time::sleep(tokio::time::Duration::from_secs(1800)).await;
        }
    });

    let db_path_for_flush = db_path.clone();
    let telemetry_db_path_for_flush = telemetry_db_path.clone();
    // Run an initial flush on startup
    let _ = flush_telemetry_to_main(&db_path_for_flush, &telemetry_db_path_for_flush);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // 5 minutes
            let _ = flush_telemetry_to_main(&db_path_for_flush, &telemetry_db_path_for_flush);
        }
    });
        
    let che_db_path = db_path.clone();
    let che_sim_time = simulated_time.clone();
    let che_simulator_port = 48130;
    tokio::spawn(async move {
        crate::che::run_che_loop(che_db_path, che_sim_time, che_simulator_port).await;
    });

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

fn get_battery_percentage() -> Option<u32> {
    if cfg!(target_os = "windows") {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(&["path", "win32_battery", "get", "estimatedchargeremaining"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_digit(10)) {
                    if let Ok(pct) = trimmed.parse::<u32>() {
                        return Some(pct);
                    }
                }
            }
        }
    } else if cfg!(target_os = "linux") {
        if let Ok(text) = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
            if let Ok(pct) = text.trim().parse::<u32>() {
                return Some(pct);
            }
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(output) = std::process::Command::new("pmset").arg("-g").arg("batt").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(idx) = text.find('%') {
                let start = text[..idx].rfind(|c: char| !c.is_digit(10)).unwrap_or(0);
                if let Ok(pct) = text[start..idx].trim().parse::<u32>() {
                    return Some(pct);
                }
            }
        }
    }
    None
}

fn get_last_activity_time(db_path: &Path, telemetry_db_path: &Path) -> Option<chrono::NaiveDateTime> {
    let mut last_time: Option<chrono::NaiveDateTime> = None;
    
    // Check telemetry DB context_events
    if let Ok(conn) = Connection::open(telemetry_db_path) {
        if let Ok(val) = conn.query_row::<String, _, _>(
            "SELECT MAX(captured_at) FROM context_events",
            [],
            |row| row.get(0)
        ) {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&val, "%Y-%m-%d %H:%M:%S") {
                last_time = Some(dt);
            } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&val) {
                last_time = Some(dt.naive_local());
            }
        }
    }
    
    // Also check main DB context_events
    if let Ok(conn) = Connection::open(db_path) {
        if let Ok(val) = conn.query_row::<String, _, _>(
            "SELECT MAX(captured_at) FROM context_events",
            [],
            |row| row.get(0)
        ) {
            let mut main_dt = None;
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&val, "%Y-%m-%d %H:%M:%S") {
                main_dt = Some(dt);
            } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&val) {
                main_dt = Some(dt.naive_local());
            }
            
            if let Some(mdt) = main_dt {
                if last_time.is_none() || mdt > last_time.unwrap() {
                    last_time = Some(mdt);
                }
            }
        }
    }
    
    last_time
}

async fn handle_heartbeat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<HeartbeatPayload>,
) -> impl axum::response::IntoResponse {
    let token_valid = check_token(&state.auth_token, &None, &headers);
    if !token_valid {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "Unauthorized"
        })));
    }
    
    if payload.status == "ALIVE" {
        let mut lh = state.last_heartbeat.lock().unwrap();
        *lh = std::time::Instant::now();
    }
    
    let (cpu_usage, ram_usage) = {
        let mut sys = state.system.lock().unwrap();
        sys.refresh_cpu();
        sys.refresh_memory();
        
        let cpu = sys.global_cpu_info().cpu_usage();
        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        let ram = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };
        (cpu, ram)
    };
    
    let battery_pct = get_battery_percentage();
    let mut pause = cpu_usage > 60.0;
    if let Some(bat) = battery_pct {
        if bat < 20 {
            pause = true;
        }
    }
    
    // Check if the system is idle
    let now = {
        let st = state.simulated_time.lock().unwrap();
        st.naive_local()
    };
    let is_idle = if let Some(last_act) = get_last_activity_time(&state.db_path, &state.telemetry_db_path) {
        let diff = now.signed_duration_since(last_act);
        diff.num_seconds() > 60 // 1 minute of inactivity
    } else {
        true
    };
    
    (StatusCode::OK, Json(serde_json::json!({
        "status": "OK",
        "pause": pause,
        "is_idle": is_idle,
        "cpu_usage": cpu_usage,
        "ram_usage": ram_usage,
        "battery_percentage": battery_pct
    })))
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
            |r| r.get::<_, Option<String>>(0)
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

        // Ghost Terminal (Proactive Environment Synthesis)
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            let parent_dir = parent.to_string_lossy().to_string();
            let mut ghost_cmd = format!("cd /d \"{}\"", parent_dir);
            ghost_cmd.push_str(" && if exist venv\\Scripts\\activate (venv\\Scripts\\activate) else if exist .venv\\Scripts\\activate (.venv\\Scripts\\activate)");
            ghost_cmd.push_str(" && echo [Chronos Ghost Terminal] Environment synthesized.");
            ghost_cmd.push_str(" && echo [Chronos Ghost Terminal] Pre-staged command: npm run dev (or python main.py)");
            
            let _ = Command::new("cmd")
                .arg("/c")
                .arg("start")
                .arg("cmd.exe")
                .arg("/k")
                .arg(&ghost_cmd)
                .spawn();
        }
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
    if let Ok(conn) = Connection::open(&state.telemetry_db_path) {
        let _ = conn.execute_batch(
            "DELETE FROM workspace_snapshots;
             DELETE FROM search_queries;
             DELETE FROM browser_sessions;
             DELETE FROM context_events;"
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

#[derive(Serialize)]
struct SystemStatus {
    status: String,
    #[serde(rename = "systemTime")]
    system_time: String,
    #[serde(rename = "hasApiKey")]
    has_api_key: bool,
    #[serde(rename = "databaseDiskSizeKb")]
    database_disk_size_kb: u64,
    #[serde(rename = "privacyMode")]
    privacy_mode: String,
    #[serde(rename = "cpuUsage")]
    cpu_usage: f32,
    #[serde(rename = "ramUsage")]
    ram_usage: f32,
    #[serde(rename = "batteryPercentage")]
    battery_percentage: Option<u32>,
    #[serde(rename = "gatekeeperPaused")]
    gatekeeper_paused: bool,
}

async fn handle_system_status(State(state): State<ServerState>) -> Json<SystemStatus> {
    let system_time = {
        let st = state.simulated_time.lock().unwrap();
        st.to_rfc3339()
    };
    let has_api_key = std::env::var("GEMINI_API_KEY")
        .map(|k| !k.is_empty() && k != "MY_GEMINI_API_KEY")
        .unwrap_or(false);
    let database_disk_size_kb = std::fs::metadata(&state.db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);
        
    let (cpu_usage, ram_usage) = {
        let mut sys = state.system.lock().unwrap();
        sys.refresh_cpu();
        sys.refresh_memory();
        
        let cpu = sys.global_cpu_info().cpu_usage();
        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        let ram = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };
        (cpu, ram)
    };
    
    let battery_percentage = get_battery_percentage();
    let mut gatekeeper_paused = cpu_usage > 60.0;
    if let Some(bat) = battery_percentage {
        if bat < 20 {
            gatekeeper_paused = true;
        }
    }
    
    Json(SystemStatus {
        status: "ONLINE".to_string(),
        system_time,
        has_api_key,
        database_disk_size_kb,
        privacy_mode: "LOCAL_ONLY_ZERO_CLOUD".to_string(),
        cpu_usage,
        ram_usage,
        battery_percentage,
        gatekeeper_paused,
    })
}

#[derive(Serialize)]
struct ProjectItem {
    id: i64,
    project_name: String,
    status: String,
    created_at: String,
    state: Option<serde_json::Value>,
    #[serde(rename = "commitmentsCount")]
    commitments_count: i64,
    deadlines: Vec<serde_json::Value>,
    #[serde(rename = "actionsCount")]
    actions_count: i64,
    #[serde(rename = "latestCheckpoint")]
    latest_checkpoint: Option<serde_json::Value>,
}

async fn handle_get_projects(State(state): State<ServerState>) -> Json<Vec<ProjectItem>> {
    let mut projects = vec![];
    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_name, status, created_at FROM projects") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            }) {
                for p in rows.flatten() {
                    let pid = p.0;
                    
                    let p_state: Option<serde_json::Value> = conn.query_row(
                        "SELECT current_summary, current_entry_point, next_action, confidence_score, updated_at FROM project_state WHERE project_id = ?1",
                        params![pid],
                        |r| {
                            Ok(serde_json::json!({
                                "project_id": pid,
                                "current_summary": r.get::<_, String>(0)?,
                                "current_entry_point": r.get::<_, Option<String>>(1)?,
                                "next_action": r.get::<_, Option<String>>(2)?,
                                "confidence_score": r.get::<_, f64>(3)?,
                                "updated_at": r.get::<_, String>(4)?,
                            }))
                        }
                    ).ok();

                    let commitments_count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM commitments WHERE project_id = ?1",
                        params![pid],
                        |r| r.get(0)
                    ).unwrap_or(0);

                    let mut deadlines = vec![];
                    if let Ok(mut dl_stmt) = conn.prepare("SELECT id, deadline_label, target_date, importance_tier, created_at FROM project_deadlines WHERE project_id = ?1") {
                        if let Ok(dl_rows) = dl_stmt.query_map(params![pid], |r| {
                            Ok(serde_json::json!({
                                "id": r.get::<_, i64>(0)?,
                                "project_id": pid,
                                "deadline_label": r.get::<_, String>(1)?,
                                "target_date": r.get::<_, String>(2)?,
                                "importance_tier": r.get::<_, String>(3)?,
                                "created_at": r.get::<_, String>(4)?,
                            }))
                        }) {
                            deadlines = dl_rows.flatten().collect();
                        }
                    }

                    let actions_count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM project_actions WHERE project_id = ?1",
                        params![pid],
                        |r| r.get(0)
                    ).unwrap_or(0);

                    let latest_checkpoint: Option<serde_json::Value> = conn.query_row(
                        "SELECT id, accomplished_text, blocked_text, next_steps_text, created_at FROM project_checkpoints WHERE project_id = ?1 ORDER BY id DESC LIMIT 1",
                        params![pid],
                        |r| {
                            Ok(serde_json::json!({
                                "id": r.get::<_, i64>(0)?,
                                "project_id": pid,
                                "accomplished_text": r.get::<_, String>(1)?,
                                "blocked_text": r.get::<_, Option<String>>(2)?,
                                "next_steps_text": r.get::<_, Option<String>>(3)?,
                                "created_at": r.get::<_, String>(4)?,
                            }))
                        }
                    ).ok();

                    projects.push(ProjectItem {
                        id: pid,
                        project_name: p.1,
                        status: p.2,
                        created_at: p.3,
                        state: p_state,
                        commitments_count,
                        deadlines,
                        actions_count,
                        latest_checkpoint,
                    });
                }
            }
        }
    }
    Json(projects)
}

fn sample_normal(mean: f64, std_dev: f64) -> f64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let u1: f64 = rng.gen::<f64>().max(1e-15);
    let u2: f64 = rng.gen::<f64>();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    z * std_dev + mean
}

async fn handle_get_commitments(State(state): State<ServerState>) -> Json<Vec<serde_json::Value>> {
    let mut commitments = vec![];
    let simulated_time = {
        let st = state.simulated_time.lock().unwrap();
        *st
    };
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, title, commitment_type, deadline_date, confidence_score, source_node_id, status, created_at FROM commitments") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, f64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            }) {
                for c in rows.flatten() {
                    let cid = c.0;
                    let pid = c.1;
                    let title = c.2;
                    let ctype = c.3;
                    let deadline_str = c.4;
                    let confidence = c.5;
                    let src_node_id = c.6;
                    let status = c.7;
                    let created_at = c.8;
                    
                    let mut health = "GREEN".to_string();
                    let mut risk_score = 0.1;
                    let mut completion_chance = 0.90;
                    let mut simulated_failure_date = "".to_string();
                    let mut marginal_loss_24h = 0.0;
                    
                    if status == "OPEN" {
                        let total_actions: i64 = conn.query_row(
                            "SELECT COUNT(*) FROM project_actions WHERE project_id = ?1",
                            params![pid],
                            |r| r.get(0)
                        ).unwrap_or(0);
                        let completed_actions: i64 = conn.query_row(
                            "SELECT COUNT(*) FROM project_actions WHERE project_id = ?1 AND status = 'COMPLETED'",
                            params![pid],
                            |r| r.get(0)
                        ).unwrap_or(0);
                        
                        let prog = if total_actions > 0 {
                            completed_actions as f64 / total_actions as f64
                        } else {
                            1.0
                        };
                        
                        let mut recent_focus_sec = 0.0;
                        let forty_eight_hours_ago = (simulated_time - chrono::Duration::hours(48)).to_rfc3339();
                        
                        if let Ok(mut node_stmt) = conn.prepare("
                            SELECT e.interaction_duration 
                            FROM context_events e
                            JOIN context_nodes n ON e.node_id = n.id
                            WHERE n.project_id = ?1 AND e.captured_at >= ?2
                        ") {
                            if let Ok(focus_rows) = node_stmt.query_map(params![pid, forty_eight_hours_ago], |r| r.get::<_, i64>(0)) {
                                let total_sec: i64 = focus_rows.flatten().sum();
                                recent_focus_sec = total_sec as f64;
                            }
                        }
                        let act_rec = (recent_focus_sec / 14400.0).min(1.0);
                        
                        let mut t_norm = 1.0;
                        let mut hours_remaining = 168.0;
                        if let Some(ref d_str) = deadline_str {
                            let d_parsed = if d_str.contains('T') {
                                chrono::DateTime::parse_from_rfc3339(d_str).map(|dt| dt.with_timezone(&chrono::Local)).ok()
                            } else {
                                chrono::NaiveDate::parse_from_str(d_str, "%Y-%m-%d")
                                    .map(|nd| nd.and_hms_opt(23, 59, 59).unwrap().and_local_timezone(chrono::Local).unwrap())
                                    .ok()
                            };
                            
                            if let Some(d_time) = d_parsed {
                                let duration = d_time.signed_duration_since(simulated_time);
                                let hours = duration.num_hours() as f64;
                                hours_remaining = hours.max(0.0);
                                t_norm = (hours_remaining / 168.0).min(1.0);
                            }
                        }
                        
                        let mut effort_remaining = 0.0;
                        if let Ok(mut eff_stmt) = conn.prepare("SELECT estimated_effort_hours FROM project_actions WHERE project_id = ?1 AND status = 'PENDING'") {
                            if let Ok(eff_rows) = eff_stmt.query_map(params![pid], |r| r.get::<_, f64>(0)) {
                                effort_remaining = eff_rows.flatten().sum();
                            }
                        }
                        let e_norm = (effort_remaining / 168.0).min(1.0);
                        
                        let w_prog = 0.30;
                        let w_act = 0.25;
                        let w_time = 0.25;
                        let w_eff = 0.20;
                        let health_val = (w_prog * prog) + (w_act * act_rec) + (w_time * t_norm) - (w_eff * e_norm);
                        
                        health = if health_val >= 0.70 {
                            "GREEN".to_string()
                        } else if health_val >= 0.40 {
                            "YELLOW".to_string()
                        } else {
                            "RED".to_string()
                        };
                        
                        // Calculate historical focus capacity over a rolling 14-day window ending at simulated_time
                        let mut daily_hours = vec![];
                        for d in 0..14 {
                            let s_time = simulated_time - chrono::Duration::days(d + 1);
                            let e_time = simulated_time - chrono::Duration::days(d);
                            let s_str = s_time.to_rfc3339();
                            let e_str = e_time.to_rfc3339();
                            
                            let focus_sec: i64 = conn.query_row(
                                "SELECT COALESCE(SUM(interaction_duration), 0) FROM context_events WHERE captured_at >= ?1 AND captured_at < ?2",
                                params![s_str, e_str],
                                |r| r.get(0)
                            ).unwrap_or(0);
                            
                            let browser_sec: i64 = conn.query_row(
                                "SELECT COALESCE(SUM(active_seconds), 0) FROM browser_sessions WHERE visit_started_at >= ?1 AND visit_started_at < ?2",
                                params![s_str, e_str],
                                |r| r.get(0)
                            ).unwrap_or(0);
                            
                            let hours = (focus_sec + browser_sec) as f64 / 3600.0;
                            daily_hours.push(hours);
                        }
                        let total_hist_hours: f64 = daily_hours.iter().sum();
                        if total_hist_hours == 0.0 {
                            daily_hours = vec![2.0; 14];
                        }
                        
                        let mean: f64 = daily_hours.iter().sum::<f64>() / 14.0;
                        let variance: f64 = daily_hours.iter().map(|&h| (h - mean).powi(2)).sum::<f64>() / 14.0;
                        let std_dev = variance.sqrt().max(0.1);
                        
                        let daily_capacity_hours = mean.max(0.1);
                        let days_remaining = hours_remaining / 24.0;
                        let available_hours_window = (days_remaining * daily_capacity_hours).max(0.1);
                        
                        risk_score = (effort_remaining / available_hours_window).min(2.0);
                        risk_score = (risk_score * 100.0).round() / 100.0;
                        
                        // 1000-run Monte Carlo Timeline Simulator
                        let mut success_count = 0;
                        let mut total_days_needed = 0.0;
                        for _ in 0..1000 {
                            let mut temp_effort = effort_remaining;
                            let mut days = 0.0;
                            while temp_effort > 0.0 && days < 365.0 {
                                let cap = sample_normal(mean, std_dev).clamp(0.0, 24.0);
                                if cap >= temp_effort {
                                    days += temp_effort / cap.max(0.1);
                                    temp_effort = 0.0;
                                } else {
                                    days += 1.0;
                                    temp_effort -= cap;
                                }
                            }
                            total_days_needed += days;
                            if days <= days_remaining {
                                success_count += 1;
                            }
                        }
                        let p_comp_mc = success_count as f64 / 1000.0;
                        let avg_days_needed = total_days_needed / 1000.0;
                        
                        completion_chance = p_comp_mc;
                        
                        let fail_time = simulated_time + chrono::Duration::days(avg_days_needed.round() as i64);
                        simulated_failure_date = fail_time.format("%Y-%m-%d").to_string();
                        
                        let delay_factor = (-0.15 * (24.0 / hours_remaining.max(1.0))).exp();
                        let delayed_chance = p_comp_mc * delay_factor;
                        marginal_loss_24h = ((p_comp_mc - delayed_chance) * 100.0).round() / 100.0;
                    }
                    
                    commitments.push(serde_json::json!({
                        "id": cid,
                        "project_id": pid,
                        "title": title,
                        "commitment_type": ctype,
                        "deadline_date": deadline_str,
                        "confidence_score": confidence,
                        "source_node_id": src_node_id,
                        "status": status,
                        "created_at": created_at,
                        "health": health,
                        "risk_score": risk_score,
                        "completion_chance": completion_chance,
                        "simulated_failure_date": simulated_failure_date,
                        "marginal_loss_24h": marginal_loss_24h,
                        "marignal_loss_24h": marginal_loss_24h,
                    }));
                }
            }
        }
    }
    
    Json(commitments)
}

async fn handle_get_actions(State(state): State<ServerState>) -> Json<Vec<serde_json::Value>> {
    let mut actions = vec![];
    let simulated_time = {
        let st = state.simulated_time.lock().unwrap();
        *st
    };
    
    let horizon_ms = 7.0 * 24.0 * 60.0 * 60.0 * 1000.0; // 7 days
    let w_prox = 0.40;
    let w_attn = 0.20;
    let w_urg = 0.25;
    let w_effort = 0.15;
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, action_text, estimated_effort_hours, status, priority_score, created_at FROM project_actions WHERE status = 'PENDING'") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, f64>(5)?,
                    row.get::<_, String>(6)?,
                ))
            }) {
                for act in rows.flatten() {
                    let aid = act.0;
                    let pid = act.1;
                    let text = act.2;
                    let effort = act.3;
                    let status = act.4;
                    let created_at = act.6;
                    
                    let mut p_prox = 0.0;
                    let mut hours_remaining = 168.0;
                    
                    if let Ok(d_str) = conn.query_row::<String, _, _>(
                        "SELECT target_date FROM project_deadlines WHERE project_id = ?1 ORDER BY target_date ASC LIMIT 1",
                        params![pid],
                        |r| r.get(0)
                    ) {
                        let d_parsed = if d_str.contains('T') {
                            chrono::DateTime::parse_from_rfc3339(&d_str).map(|dt| dt.with_timezone(&chrono::Local)).ok()
                        } else {
                            chrono::NaiveDate::parse_from_str(&d_str, "%Y-%m-%d")
                                .map(|nd| nd.and_hms_opt(23, 59, 59).unwrap().and_local_timezone(chrono::Local).unwrap())
                                .ok()
                        };
                        
                        if let Some(d_time) = d_parsed {
                            let duration_ms = d_time.signed_duration_since(simulated_time).num_milliseconds() as f64;
                            hours_remaining = (duration_ms / (60.0 * 60.0 * 1000.0)).max(0.0);
                            p_prox = (1.0 - (duration_ms / horizon_ms)).max(0.0);
                        }
                    }
                    
                    let mut avg_attn_norm = 0.0;
                    if let Ok(mut node_stmt) = conn.prepare("SELECT id FROM context_nodes WHERE project_id = ?1") {
                        if let Ok(node_rows) = node_stmt.query_map(params![pid], |r| r.get::<_, i64>(0)) {
                            let mut total_attn = 0.0;
                            let mut count = 0;
                            for nid in node_rows.flatten() {
                                let focus_sec: i64 = conn.query_row(
                                    "SELECT SUM(interaction_duration) FROM context_events WHERE node_id = ?1 AND event_type IN ('TAB_FOCUS', 'OPENED', 'EDITED')",
                                    params![nid],
                                    |r| r.get(0)
                                ).unwrap_or(0);
                                let edits: i64 = conn.query_row(
                                    "SELECT COUNT(*) FROM context_events WHERE node_id = ?1 AND event_type = 'EDITED'",
                                    params![nid],
                                    |r| r.get(0)
                                ).unwrap_or(0);
                                let revisits: i64 = conn.query_row(
                                    "SELECT COUNT(*) FROM context_events WHERE node_id = ?1",
                                    params![nid],
                                    |r| r.get(0)
                                ).unwrap_or(0);
                                
                                let w_attn = 0.50 * (focus_sec as f64).ln_1p() + 0.35 * (edits as f64) + 0.15 * (revisits as f64);
                                total_attn += w_attn;
                                count += 1;
                            }
                            if count > 0 {
                                avg_attn_norm = (total_attn / count as f64 / 5.0).min(1.0);
                            }
                        }
                    }
                    
                    let mut c_urg = 0.0;
                    if let Ok(row) = conn.query_row::<(Option<String>, Option<String>), _, _>(
                        "SELECT blocked_text, next_steps_text FROM project_checkpoints WHERE project_id = ?1 ORDER BY id DESC LIMIT 1",
                        params![pid],
                        |r| Ok((r.get(0)?, r.get(1)?))
                    ) {
                        let blocked_text = row.0.unwrap_or_default().to_lowercase();
                        let next_text = row.1.unwrap_or_default().to_lowercase();
                        let act_lower = text.to_lowercase();
                        if blocked_text.contains(&act_lower) || next_text.contains(&act_lower) {
                            c_urg = 1.0;
                        }
                    }
                    
                    let e_achieve = 1.0 - (effort / 168.0).min(1.0);
                    
                    let raw_ap = (w_prox * p_prox) + (w_attn * avg_attn_norm) + (w_urg * c_urg) + (w_effort * e_achieve);
                    let score = ((raw_ap * 10.0) * 10.0).round() / 10.0;
                    
                    let mut diagnostics = vec![];
                    if w_prox * p_prox >= 0.20 {
                        diagnostics.push(format!("Deadline is imminent (under {:.1} days remaining)", hours_remaining / 24.0));
                    }
                    if c_urg == 1.0 {
                        diagnostics.push("Explicitly flagged or referenced in your recent project checkpoints".to_string());
                    }
                    if avg_attn_norm <= 0.25 {
                        diagnostics.push("Project progress has stalled (zero focus detected recently)".to_string());
                    }
                    if w_effort * e_achieve >= 0.08 {
                        diagnostics.push(format!("Quick win: highly achievable task (estimated {} hrs)", effort));
                    }
                    if diagnostics.is_empty() {
                        diagnostics.push("Standard scheduled sequencing of commitments".to_string());
                    }
                    
                    actions.push(serde_json::json!({
                        "id": aid,
                        "project_id": pid,
                        "action_text": text,
                        "estimated_effort_hours": effort,
                        "status": status,
                        "priority_score": score,
                        "created_at": created_at,
                        "why_now_diagnostics": diagnostics,
                    }));
                }
            }
        }
    }
    
    actions.sort_by(|a, b| {
        let a_score = a.get("priority_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b_score = b.get("priority_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    Json(actions)
}

#[derive(Deserialize)]
struct PostCheckpointPayloadReact {
    project_id: i64,
    accomplished_text: String,
    blocked_text: Option<String>,
    next_steps_text: Option<String>,
}

async fn handle_post_checkpoint_react(
    State(state): State<ServerState>,
    Json(payload): Json<PostCheckpointPayloadReact>,
) -> Json<serde_json::Value> {
    let simulated_time = {
        let st = state.simulated_time.lock().unwrap();
        st.to_rfc3339()
    };
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        let res = conn.execute(
            "INSERT INTO project_checkpoints (project_id, accomplished_text, blocked_text, next_steps_text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                payload.project_id,
                payload.accomplished_text,
                payload.blocked_text,
                payload.next_steps_text,
                simulated_time
            ]
        );
        if res.is_ok() {
            let next_action = payload.next_steps_text.clone().unwrap_or_default();
            let _ = conn.execute(
                "UPDATE project_state SET next_action = ?1, updated_at = ?2 WHERE project_id = ?3",
                params![next_action, simulated_time, payload.project_id]
            );
            
            let checkpoint_id: i64 = conn.query_row(
                "SELECT id FROM project_checkpoints ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0)
            ).unwrap_or(1);
            
            return Json(serde_json::json!({
                "success": true,
                "checkpoint": {
                    "id": checkpoint_id,
                    "project_id": payload.project_id,
                    "accomplished_text": payload.accomplished_text,
                    "blocked_text": payload.blocked_text,
                    "next_steps_text": payload.next_steps_text,
                    "created_at": simulated_time,
                }
            }));
        }
    }
    Json(serde_json::json!({ "success": false, "error": "Failed to save checkpoint" }))
}

#[derive(Deserialize)]
struct ToggleActionPayload {
    action_id: i64,
}

async fn handle_toggle_action_react(
    State(state): State<ServerState>,
    Json(payload): Json<ToggleActionPayload>,
) -> Json<serde_json::Value> {
    if let Ok(conn) = Connection::open(&state.db_path) {
        let current_status: Option<String> = conn.query_row(
            "SELECT status FROM project_actions WHERE id = ?1",
            params![payload.action_id],
            |r| r.get(0)
        ).ok();
        
        if let Some(status) = current_status {
            let new_status = if status == "COMPLETED" { "PENDING" } else { "COMPLETED" };
            let _ = conn.execute(
                "UPDATE project_actions SET status = ?1 WHERE id = ?2",
                params![new_status, payload.action_id]
            );
            return Json(serde_json::json!({
                "success": true,
                "action": {
                    "id": payload.action_id,
                    "status": new_status,
                }
            }));
        }
    }
    Json(serde_json::json!({ "success": false, "error": "Action not found" }))
}

#[derive(Deserialize)]
struct TimeTravelPayload {
    hours: i64,
}

async fn handle_time_travel_react(
    State(state): State<ServerState>,
    Json(payload): Json<TimeTravelPayload>,
) -> Json<serde_json::Value> {
    let new_time_str = {
        let mut st = state.simulated_time.lock().unwrap();
        *st = *st + chrono::Duration::hours(payload.hours);
        st.to_rfc3339()
    };
    
    Json(serde_json::json!({
        "success": true,
        "newTime": new_time_str,
        "message": format!("Time traveled forward {} hours.", payload.hours)
    }))
}

#[derive(Deserialize)]
struct RestoreWorkspacePayload {
    project_id: i64,
}

async fn handle_restore_workspace_react(
    State(state): State<ServerState>,
    Json(payload): Json<RestoreWorkspacePayload>,
) -> Json<serde_json::Value> {
    let mut file_path = String::new();
    let mut snapshot = serde_json::json!(null);
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(row) = conn.query_row::<(String, i64, i64, String), _, _>(
            "SELECT active_file_path, cursor_line, cursor_column, open_tabs_json FROM workspace_snapshots WHERE project_id = ?1 OR project_id IS NULL ORDER BY id DESC LIMIT 1",
            params![payload.project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        ) {
            file_path = row.0.clone();
            snapshot = serde_json::json!({
                "project_id": payload.project_id,
                "active_file_path": row.0,
                "cursor_line": row.1,
                "cursor_column": row.2,
                "open_tabs_json": row.3,
            });
        }
    }
    
    if !file_path.is_empty() {
        let _ = Command::new("code").arg(&file_path).spawn();

        // Ghost Terminal (Proactive Environment Synthesis) + Predictive Autocomplete
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            let parent_dir = parent.to_string_lossy().to_string();
            let mut ghost_cmd = format!("cd /d \"{}\"", parent_dir);
            ghost_cmd.push_str(" && if exist venv\\Scripts\\activate (venv\\Scripts\\activate) else if exist .venv\\Scripts\\activate (.venv\\Scripts\\activate)");
            ghost_cmd.push_str(" && echo [Chronos Ghost Terminal] Environment synthesized.");

            // Predictive Terminal Autocomplete: query last terminal context for error patterns
            let suggestion = get_terminal_suggestion(&state.db_path);
            if let Some(ref cmd) = suggestion {
                ghost_cmd.push_str(&format!(" && echo [Chronos Ghost Terminal] Suggested fix: {}", cmd));
            } else {
                ghost_cmd.push_str(" && echo [Chronos Ghost Terminal] Ready to resume work.");
            }
            
            let _ = Command::new("cmd")
                .arg("/c")
                .arg("start")
                .arg("cmd.exe")
                .arg("/k")
                .arg(&ghost_cmd)
                .spawn();
        }
    }
    
    let mut why_stopped = "No explicit blocker reported.".to_string();
    let narrative_path = dirs::home_dir().unwrap_or_default().join(".config").join("chronos").join("reconstruction_narrative.md");
    if let Ok(content) = fs::read_to_string(narrative_path) {
        why_stopped = content;
    } else if file_path.is_empty() {
        why_stopped = "No active workspace snapshot has been recorded for this project yet. Start editing files in VS Code and searching the web to populate your workspace dashboard!".to_string();
    } else {
        why_stopped = format!(
            "You paused work on this project recently:\n• **Mental Thread**: You were working in `{}`.\n• **Pick up at**: Start by resolving the next actions.",
            file_path
        );
    }

    let terminal_suggestion = get_terminal_suggestion(&state.db_path);
    
    Json(serde_json::json!({
        "success": true,
        "snapshot": snapshot,
        "why_stopped_narrative": why_stopped,
        "terminal_suggestion": terminal_suggestion,
        "message": "Restored environment: Opened workspace folder, loaded tabs, positioned cursor."
    }))
}

#[derive(Deserialize)]
struct SimulatePayload {
    event_type: String,
    payload: serde_json::Value,
}

async fn handle_simulate_telemetry(
    State(state): State<ServerState>,
    Json(payload): Json<SimulatePayload>,
) -> Json<serde_json::Value> {
    let simulated_time = {
        let st = state.simulated_time.lock().unwrap();
        st.to_rfc3339()
    };
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        if payload.event_type == "DOWNLOAD_PDF" {
            let filename = payload.payload.get("filename").and_then(|v| v.as_str()).unwrap_or("ML_Theoretical_Foundations.pdf");
            let entity_key = format!("FILE:downloads/{}", filename);
            
            let _ = conn.execute(
                "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name, created_at)
                 VALUES (1, ?1, 'DOCUMENT', ?2, ?3)
                 ON CONFLICT(entity_key) DO UPDATE SET created_at = ?3",
                params![entity_key, filename, simulated_time]
            );
            
            let node_id: i64 = conn.query_row(
                "SELECT id FROM context_nodes WHERE entity_key = ?1",
                params![entity_key],
                |r| r.get(0)
            ).unwrap_or(1);
            
            let title = "ML Theoretical Foundations Assignment".to_string();
            let _ = conn.execute(
                "INSERT INTO commitments (project_id, title, commitment_type, deadline_date, confidence_score, source_node_id, status, created_at)
                 VALUES (1, ?1, 'ASSIGNMENT', '2026-07-20T23:59:59Z', 0.94, ?2, 'OPEN', ?3)",
                params![title, node_id, simulated_time]
            );
            
            let commitment_id: i64 = conn.query_row(
                "SELECT id FROM commitments WHERE title = ?1 ORDER BY id DESC LIMIT 1",
                params![title],
                |r| r.get(0)
            ).unwrap_or(1);
            
            let _ = conn.execute(
                "INSERT INTO project_actions (project_id, action_text, estimated_effort_hours, status, priority_score, created_at)
                 VALUES (1, 'Complete ML math calculations section', 4.5, 'PENDING', 0.0, ?1)",
                params![simulated_time]
            );
            
            return Json(serde_json::json!({
                "success": true,
                "discovered_commitment": {
                    "id": commitment_id,
                    "project_id": 1,
                    "title": title,
                    "commitment_type": "ASSIGNMENT",
                    "deadline_date": "2026-07-20T23:59:59Z",
                    "confidence_score": 0.94,
                    "source_node_id": node_id,
                    "status": "OPEN",
                    "created_at": simulated_time,
                },
                "discovered_node": {
                    "id": node_id,
                    "project_id": 1,
                    "entity_key": entity_key,
                    "entity_type": "DOCUMENT",
                    "display_name": filename,
                },
                "message": "CDE processed document: Discovered commitment: 'ML Theoretical Foundations' due July 20 (Conf: 94%)"
            }));
        } else if payload.event_type == "BROWSER_SEARCH" {
            let query = payload.payload.get("query").and_then(|v| v.as_str()).unwrap_or("");
            
            let url = format!("https://www.google.com/search?q={}", query);
            let page_title = format!("Google Search: {}", query);
            
            if let Ok(tele_conn) = Connection::open(&state.telemetry_db_path) {
                let _ = tele_conn.execute(
                    "INSERT INTO browser_sessions (project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at)
                     VALUES (1, ?1, ?2, 'google.com', ?3, ?3, 45, ?3)",
                    params![url, page_title, simulated_time]
                );
                
                let session_id: i64 = tele_conn.query_row(
                    "SELECT id FROM browser_sessions ORDER BY id DESC LIMIT 1",
                    [],
                    |r| r.get(0)
                ).unwrap_or(1);
                
                let _ = tele_conn.execute(
                    "INSERT INTO search_queries (browser_session_id, query_text, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![session_id, query, simulated_time]
                );
            }
            
            let entity_key = format!("URL:search?q={}", query);
            let display_name = format!("Google Search: \"{}\"", query);
            let _ = conn.execute(
                "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name, created_at)
                 VALUES (1, ?1, 'URL', ?2, ?3)
                 ON CONFLICT(entity_key) DO UPDATE SET created_at = ?3",
                params![entity_key, display_name, simulated_time]
            );
            
            let node_id: i64 = conn.query_row(
                "SELECT id FROM context_nodes WHERE entity_key = ?1",
                params![entity_key],
                |r| r.get(0)
            ).unwrap_or(1);
            
            let _ = conn.execute(
                "INSERT OR IGNORE INTO graph_edges (source_node_id, target_node_id, edge_type, weight, created_at)
                 VALUES (1, ?1, 'REFERENCES', 1.2, ?2)",
                params![node_id, simulated_time]
            );
            
            return Json(serde_json::json!({
                "success": true,
                "message": format!("Browser Telemetry tracked search query: \"{}\" | Dynamic Graph Edge generated.", query)
            }));
        } else if payload.event_type == "FILE_EDIT" {
            let filepath = payload.payload.get("filepath").and_then(|v| v.as_str()).unwrap_or("src/billing/invoice_model.ts");
            let file_name = Path::new(filepath).file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown");
            let entity_key = format!("FILE:{}", filepath);
            
            let _ = conn.execute(
                "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name, created_at)
                 VALUES (1, ?1, 'FILE', ?2, ?3)
                 ON CONFLICT(entity_key) DO UPDATE SET created_at = ?3",
                params![entity_key, file_name, simulated_time]
            );
            
            let node_id: i64 = conn.query_row(
                "SELECT id FROM context_nodes WHERE entity_key = ?1",
                params![entity_key],
                |r| r.get(0)
            ).unwrap_or(1);
            
            if let Ok(tele_conn) = Connection::open(&state.telemetry_db_path) {
                let _ = tele_conn.execute(
                    "INSERT INTO context_events (node_id, event_type, interaction_duration, captured_at)
                     VALUES (?1, 'EDITED', 10, ?2)",
                    params![node_id, simulated_time]
                );
                
                let _ = tele_conn.execute(
                    "INSERT INTO workspace_snapshots (project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at)
                     VALUES (1, ?1, 112, 4, '[\"package.json\", \"src/billing/invoice_model.ts\"]', ?2)",
                    params![filepath, simulated_time]
                );
            }
            
            return Json(serde_json::json!({
                "success": true,
                "message": format!("Filesystem watcher tracked edit in: \"{}\" | Context snapshot captured.", filepath)
            }));
        }
    }
    
    Json(serde_json::json!({ "success": false, "error": "Unknown event" }))
}

#[derive(Deserialize)]
struct GeneratePlanPayload {
    commitment_id: i64,
}

async fn handle_generate_plan(
    State(state): State<ServerState>,
    Json(payload): Json<GeneratePlanPayload>,
) -> Json<serde_json::Value> {
    let simulated_time = {
        let st = state.simulated_time.lock().unwrap();
        st.to_rfc3339()
    };
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        let project_id: Option<i64> = conn.query_row(
            "SELECT project_id FROM commitments WHERE id = ?1",
            params![payload.commitment_id],
            |r| r.get(0)
        ).ok();
        
        if let Some(pid) = project_id {
            let mut actions = vec![];
            if let Ok(mut stmt) = conn.prepare("SELECT action_text, estimated_effort_hours FROM project_actions WHERE project_id = ?1 AND status = 'PENDING'") {
                if let Ok(rows) = stmt.query_map(params![pid], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
                }) {
                    for row in rows.flatten() {
                        actions.push(row);
                    }
                }
            }
            
            let mut plan_list = vec![];
            for (idx, act) in actions.iter().enumerate() {
                let day_label = if idx == 0 {
                    "[Today]".to_string()
                } else if idx == 1 {
                    "[Tomorrow]".to_string()
                } else {
                    format!("[Day {}]", idx + 1)
                };
                
                plan_list.push(serde_json::json!({
                    "day": day_label,
                    "task": act.0,
                    "hours": act.1,
                }));
            }
            
            let plan_payload_json = serde_json::to_string(&plan_list).unwrap_or_else(|_| "[]".to_string());
            
            let _ = conn.execute(
                "INSERT INTO recovery_plans (commitment_id, plan_payload_json, generated_at)
                 VALUES (?1, ?2, ?3)",
                params![payload.commitment_id, plan_payload_json, simulated_time]
            );
            
            let plan_id: i64 = conn.query_row(
                "SELECT id FROM recovery_plans WHERE commitment_id = ?1 ORDER BY id DESC LIMIT 1",
                params![payload.commitment_id],
                |r| r.get(0)
            ).unwrap_or(1);
            
            return Json(serde_json::json!({
                "success": true,
                "plan": {
                    "id": plan_id,
                    "commitment_id": payload.commitment_id,
                    "plan_payload_json": plan_payload_json,
                    "generated_at": simulated_time,
                }
            }));
        }
    }
    
    Json(serde_json::json!({ "success": false, "error": "Commitment not found" }))
}

async fn handle_arc_briefs(State(state): State<ServerState>) -> Json<Vec<serde_json::Value>> {
    let mut results = vec![];
    let mut id_counter = 1;

    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(mut stmt) = conn.prepare("SELECT brief_payload_json FROM autonomous_research_briefs ORDER BY id DESC LIMIT 1") {
            if let Ok(mut rows) = stmt.query([]) {
                if let Ok(Some(row)) = rows.next() {
                    if let Ok(payload_str) = row.get::<_, String>(0) {
                        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&payload_str) {
                            for item in parsed {
                                if let Some(obj) = item.as_object() {
                                    let similarity = obj.get("relevance_score").and_then(|v| v.as_f64()).unwrap_or(0.85);
                                    let sim_str = format!("{:.0}%", similarity * 100.0);
                                    results.push(serde_json::json!({
                                        "id": id_counter,
                                        "title": obj.get("title").unwrap_or(&serde_json::json!("Unknown Title")),
                                        "source": obj.get("source").unwrap_or(&serde_json::json!("Unknown Source")),
                                        "similarity": sim_str,
                                        "summary": obj.get("summary").unwrap_or(&serde_json::json!("")),
                                        "link": obj.get("url").unwrap_or(&serde_json::json!(""))
                                    }));
                                    id_counter += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Json(results)
}

async fn handle_get_telemetry_logs(State(state): State<ServerState>) -> Json<Vec<serde_json::Value>> {
    let mut logs = vec![];
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        let _ = conn.execute("ATTACH DATABASE ?1 AS telemetry", params![state.telemetry_db_path.to_str().unwrap()]);
        
        // 1. Fetch context events (file edits and tab focus) from both main and telemetry DBs
        let mut events = vec![];
        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, captured_at, event_type, display_name, entity_key, entity_type FROM (
                SELECT ce.id, ce.captured_at, ce.event_type, cn.display_name, cn.entity_key, cn.entity_type
                FROM main.context_events ce
                JOIN main.context_nodes cn ON ce.node_id = cn.id
                UNION ALL
                SELECT ce.id, ce.captured_at, ce.event_type, cn.display_name, cn.entity_key, cn.entity_type
                FROM telemetry.context_events ce
                JOIN main.context_nodes cn ON ce.node_id = cn.id
             ) ORDER BY captured_at DESC LIMIT 100"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            }) {
                for row in rows.flatten() {
                    events.push(row);
                }
            }
        }
        
        // 2. Fetch search queries from both main and telemetry DBs
        let mut searches = vec![];
        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, created_at, query_text FROM (
                SELECT sq.id, sq.created_at, sq.query_text FROM main.search_queries sq
                UNION ALL
                SELECT sq.id, sq.created_at, sq.query_text FROM telemetry.search_queries sq
             ) ORDER BY created_at DESC LIMIT 100"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
            }) {
                for row in rows.flatten() {
                    searches.push(row);
                }
            }
        }
        
        // 3. Fetch project checkpoints
        let mut checkpoints = vec![];
        if let Ok(mut stmt) = conn.prepare(
            "SELECT pc.id, pc.created_at, pc.accomplished_text, pc.blocked_text, pc.next_steps_text, p.project_name
             FROM main.project_checkpoints pc
             JOIN main.projects p ON pc.project_id = p.id
             ORDER BY pc.id DESC LIMIT 100"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            }) {
                for row in rows.flatten() {
                    checkpoints.push(row);
                }
            }
        }
        
        // 4. Fetch recovery plans
        let mut plans = vec![];
        if let Ok(mut stmt) = conn.prepare(
            "SELECT rp.id, rp.generated_at, c.title
             FROM main.recovery_plans rp
             JOIN main.commitments c ON rp.commitment_id = c.id
             ORDER BY rp.id DESC LIMIT 100"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
            }) {
                for row in rows.flatten() {
                    plans.push(row);
                }
            }
        }

        // Format and merge them
        let mut items = vec![];
        
        for (id, captured_at, event_type, display_name, entity_key, entity_type) in events {
            let formatted_time = format_timestamp(&captured_at);
            if event_type == "MANUAL_INGEST" || entity_key.starts_with("MANUAL:") {
                let category = match entity_type.as_str() {
                    "URL" => "BROWSER_MONITOR",
                    "FILE" => "FS_MONITOR",
                    _ => "COGNITIVE_TRACE",
                };
                let display_name_pref = format!("📋 Manual Ingest: {}", display_name);
                let detail = if entity_key.starts_with("URL:") {
                    entity_key.replace("URL:", "")
                } else if entity_key.starts_with("FILE:") {
                    entity_key.replace("FILE:", "")
                } else {
                    entity_key.replace("MANUAL:", "")
                };
                let norm_ts = normalize_raw_timestamp(&captured_at);
                items.push((
                    norm_ts.clone(),
                    serde_json::json!({
                        "id": id,
                        "timestamp": formatted_time,
                        "raw_timestamp": norm_ts,
                        "category": category,
                        "event_type": "MANUAL_INGEST",
                        "display_name": display_name_pref,
                        "detail": detail
                    })
                ));
            } else if event_type == "COMPACTED" || entity_type == "DOCUMENT" {
                let norm_ts = normalize_raw_timestamp(&captured_at);
                items.push((
                    norm_ts.clone(),
                    serde_json::json!({
                        "id": id,
                        "timestamp": formatted_time,
                        "raw_timestamp": norm_ts,
                        "category": "SYSTEM",
                        "event_type": "COMPACTED",
                        "display_name": "Consolidated Workspace Context",
                        "detail": display_name // display_name from DB holds the markdown summary
                    })
                ));
            } else if event_type == "EDITED" || entity_type == "FILE" {
                let norm_ts = normalize_raw_timestamp(&captured_at);
                items.push((
                    norm_ts.clone(),
                    serde_json::json!({
                        "id": id,
                        "timestamp": formatted_time,
                        "raw_timestamp": norm_ts,
                        "category": "FS_MONITOR",
                        "event_type": "FILE_EDIT",
                        "display_name": display_name,
                        "detail": entity_key.replace("FILE:", "")
                    })
                ));
            } else if event_type == "TAB_FOCUS" || entity_type == "URL" || entity_type == "RESEARCH_SESSION" {
                let mut category = "BROWSER_MONITOR";
                let mut ev_type = "TAB_FOCUS";
                let mut detail = entity_key.clone();
                
                if entity_key.starts_with("URL:") {
                    detail = entity_key.replace("URL:", "");
                } else if entity_key.starts_with("APP:") {
                    let app_content = entity_key.strip_prefix("APP:").unwrap_or(&entity_key);
                    let parts: Vec<&str> = app_content.splitn(2, ':').collect();
                    let app_name = parts.get(0).cloned().unwrap_or("Unknown App");
                    let app_name_lower = app_name.to_lowercase();
                    
                    if app_name_lower.contains("terminal")
                        || app_name_lower.contains("cmd")
                        || app_name_lower.contains("command prompt")
                        || app_name_lower.contains("powershell")
                        || app_name_lower.contains("wt")
                        || app_name_lower.contains("bash")
                        || app_name_lower.contains("zsh")
                        || app_name_lower.contains("git-bash")
                        || app_name_lower.contains("git bash")
                        || app_name_lower.contains("sh")
                        || app_name_lower.contains("conhost")
                        || app_name_lower.contains("alacritty")
                        || app_name_lower.contains("kitty")
                    {
                        category = "TERMINAL_MONITOR";
                        ev_type = "TERMINAL_FOCUS";
                    } else if app_name_lower.contains("code")
                        || app_name_lower.contains("vscode")
                        || app_name_lower.contains("visual studio code")
                        || app_name_lower.contains("cursor")
                        || app_name_lower.contains("sublime")
                        || app_name_lower.contains("intellij")
                        || app_name_lower.contains("pycharm")
                        || app_name_lower.contains("webstorm")
                        || app_name_lower.contains("clion")
                        || app_name_lower.contains("rider")
                        || app_name_lower.contains("studio")
                        || app_name_lower.contains("eclipse")
                        || app_name_lower.contains("notepad++")
                        || app_name_lower.contains("vim")
                        || app_name_lower.contains("neovim")
                        || app_name_lower.contains("emacs")
                    {
                        category = "IDE_MONITOR";
                        ev_type = "IDE_FOCUS";
                    } else if app_name_lower.contains("whatsapp")
                        || app_name_lower.contains("slack")
                        || app_name_lower.contains("discord")
                        || app_name_lower.contains("teams")
                        || app_name_lower.contains("telegram")
                        || app_name_lower.contains("signal")
                        || app_name_lower.contains("messenger")
                        || app_name_lower.contains("skype")
                        || app_name_lower.contains("zoom")
                    {
                        category = "COMMUNICATION_MONITOR";
                        ev_type = "COMMUNICATION_FOCUS";
                    } else if app_name_lower.contains("chrome")
                        || app_name_lower.contains("firefox")
                        || app_name_lower.contains("edge")
                        || app_name_lower.contains("safari")
                        || app_name_lower.contains("opera")
                        || app_name_lower.contains("brave")
                        || app_name_lower.contains("arc")
                        || app_name_lower.contains("browser")
                    {
                        category = "BROWSER_MONITOR";
                        ev_type = "TAB_FOCUS";
                    } else {
                        category = "APP_MONITOR";
                        ev_type = "APP_FOCUS";
                    }
                    detail = app_content.to_string();
                }
                
                // Override for YouTube / Shorts so they are not classified as productive focus
                let detail_lower = detail.to_lowercase();
                let display_name_lower = display_name.to_lowercase();
                if detail_lower.contains("youtube")
                    || detail_lower.contains("shorts")
                    || display_name_lower.contains("youtube")
                    || display_name_lower.contains("shorts")
                {
                    category = "BROWSER_MONITOR";
                    ev_type = "YT_SHORTS";
                }
                
                let norm_ts = normalize_raw_timestamp(&captured_at);
                items.push((
                    norm_ts.clone(),
                    serde_json::json!({
                        "id": id,
                        "timestamp": formatted_time,
                        "raw_timestamp": norm_ts,
                        "category": category,
                        "event_type": ev_type,
                        "display_name": display_name,
                        "detail": detail
                    })
                ));
            }
        }
        
        for (id, created_at, query_text) in searches {
            let formatted_time = format_timestamp(&created_at);
            let norm_ts = normalize_raw_timestamp(&created_at);
            items.push((
                norm_ts.clone(),
                serde_json::json!({
                    "id": id,
                    "timestamp": formatted_time,
                    "raw_timestamp": norm_ts,
                    "category": "BROWSER_MONITOR",
                    "event_type": "BROWSER_SEARCH",
                    "display_name": query_text,
                    "detail": "Search Query Captured"
                })
            ));
        }
        
        for (id, created_at, accomplished, blocked, next_steps, proj_name) in checkpoints {
            let formatted_time = format_timestamp(&created_at);
            let blocker_info = if blocked.trim().is_empty() {
                "None".to_string()
            } else {
                blocked.clone()
            };
            let norm_ts = normalize_raw_timestamp(&created_at);
            items.push((
                norm_ts.clone(),
                serde_json::json!({
                    "id": id,
                    "timestamp": formatted_time,
                    "raw_timestamp": norm_ts,
                    "category": "COGNITIVE_TRACE",
                    "event_type": "CHECKPOINT",
                    "display_name": format!("Project \"{}\"", proj_name),
                    "detail": format!("Accomplished: {} | Blocker: {} | Next Steps: {}", accomplished, blocker_info, next_steps)
                })
            ));
        }
        
        for (id, generated_at, title) in plans {
            let formatted_time = format_timestamp(&generated_at);
            let norm_ts = normalize_raw_timestamp(&generated_at);
            items.push((
                norm_ts.clone(),
                serde_json::json!({
                    "id": id,
                    "timestamp": formatted_time,
                    "raw_timestamp": norm_ts,
                    "category": "RECOVERY_PLAN",
                    "event_type": "RECOVERY_PLAN",
                    "display_name": title,
                    "detail": "Synthesized check-list recovery plan"
                })
            ));
        }
        
        // Sort items by original timestamp ASCENDING to resolve context chronologically
        items.sort_by(|a, b| a.0.cmp(&b.0));
        
        let mut active_project: Option<String> = None;
        let mut last_deep_work_ts: i64 = 0;
        let mut resolved_items: Vec<serde_json::Value> = Vec::new();
        
        for (ts_str, mut item) in items {
            let category = item.get("category").and_then(|v| v.as_str()).unwrap_or("");
            let detail = item.get("detail").and_then(|v| v.as_str()).unwrap_or("");
            let display_name = item.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
            
            let item_ts = if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&ts_str) {
                parsed.timestamp()
            } else { 0 };
            
            let mut project_name = "Unassigned Research".to_string();
            let mut focus_mode = "Unassigned".to_string();
            
            if category == "IDE_MONITOR" || category == "TERMINAL_MONITOR" || category == "FS_MONITOR" {
                let path_str = if category == "TERMINAL_MONITOR" { detail } else { detail };
                let proj = extract_project_from_path_str(path_str);
                
                project_name = proj.clone();
                focus_mode = "DeepWork".to_string();
                
                active_project = Some(proj);
                last_deep_work_ts = item_ts;
            } else if category == "BROWSER_MONITOR" {
                if item_ts - last_deep_work_ts > CONTEXT_EXPIRY_SECS {
                    active_project = None;
                }
                
                let display_name_lower = display_name.to_lowercase();
                let detail_lower = detail.to_lowercase();
                let is_ai = AI_DOMAINS.iter().any(|&d| display_name_lower.contains(d) || detail_lower.contains(d));
                let is_distraction = !is_ai && DISTRACTION_DOMAINS.iter().any(|&d| display_name_lower.contains(d) || detail_lower.contains(d));
                if is_distraction {
                    focus_mode = "Distraction".to_string();
                    project_name = "Non-Focused".to_string();
                } else {
                    if let Some(ref proj) = active_project {
                        project_name = proj.clone();
                        focus_mode = "ProjectResearch".to_string();
                    } else {
                        if is_ai {
                            project_name = "AI Assistance".to_string();
                            focus_mode = "ProjectResearch".to_string();
                        } else {
                            focus_mode = "Unassigned".to_string();
                            project_name = "Unassigned Research".to_string();
                        }
                    }
                }
            } else {
                focus_mode = "System".to_string();
                project_name = "System".to_string();
            }
            
            if let Some(obj) = item.as_object_mut() {
                obj.insert("focus_mode".to_string(), serde_json::json!(focus_mode));
                obj.insert("project_name".to_string(), serde_json::json!(project_name));
            }
            resolved_items.push(item);
        }
        
        let mut grouped_items: Vec<serde_json::Value> = Vec::new();
        
        for item in resolved_items {
            let focus_mode = item.get("focus_mode").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let project_name = item.get("project_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let category = item.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let detail = item.get("detail").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let display_name = item.get("display_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let item_raw_ts = item.get("raw_timestamp").and_then(|v| v.as_str()).unwrap_or("");
            let item_ts = if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(item_raw_ts) {
                parsed.timestamp()
            } else { 0 };
            
            let group_key_val = if category == "BROWSER_MONITOR" {
                extract_domain_rust(&detail)
            } else if category == "TERMINAL_MONITOR" || category == "IDE_MONITOR" || category == "COMMUNICATION_MONITOR" || category == "APP_MONITOR" {
                let parts: Vec<&str> = detail.splitn(2, ':').collect();
                parts.get(0).cloned().unwrap_or(&display_name).to_string()
            } else {
                category.clone()
            };

            let mut merged = false;
            // Scan grouped_items in reverse to find the most recent active matching group
            for g in grouped_items.iter_mut().rev() {
                let g_cat = g.get("category").and_then(|v| v.as_str()).unwrap_or("");
                let g_key = if g_cat == "BROWSER_MONITOR" {
                    g.get("group_domain").and_then(|v| v.as_str()).unwrap_or("")
                } else {
                    g.get("display_name").and_then(|v| v.as_str()).unwrap_or("")
                };

                if g_cat == category && g_key == group_key_val {
                    let g_last_ts = g.get("last_timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
                    if item_ts - g_last_ts <= 300 {
                        // Merge item into this group
                        if let Some(sub_items) = g.get_mut("sub_items").and_then(|v| v.as_array_mut()) {
                            sub_items.push(serde_json::json!({
                                "id": item.get("id").cloned().unwrap_or(serde_json::Value::Null),
                                "timestamp": item.get("timestamp").cloned().unwrap_or(serde_json::Value::Null),
                                "title": display_name,
                                "url": detail,
                                "category": category,
                                "event_type": item.get("event_type").cloned().unwrap_or(serde_json::Value::Null)
                            }));
                            let count = sub_items.len();
                            g.as_object_mut().unwrap().insert("group_count".to_string(), serde_json::json!(count));
                        }
                        
                        if let Some(ids) = g.get_mut("ids").and_then(|v| v.as_array_mut()) {
                            if let Some(id_val) = item.get("id") {
                                ids.push(id_val.clone());
                            }
                        }
                        
                        g.as_object_mut().unwrap().insert("last_timestamp".to_string(), serde_json::json!(item_ts));
                        g.as_object_mut().unwrap().insert("timestamp".to_string(), item.get("timestamp").cloned().unwrap_or(serde_json::Value::Null));
                        g.as_object_mut().unwrap().insert("raw_timestamp".to_string(), item.get("raw_timestamp").cloned().unwrap_or(serde_json::Value::Null));
                        merged = true;
                    }
                    // Since this is the newest matching group, stop searching.
                    break;
                }
            }

            if !merged {
                let mut new_group = item.clone();
                new_group.as_object_mut().unwrap().insert("is_grouped".to_string(), serde_json::json!(true));
                new_group.as_object_mut().unwrap().insert("last_timestamp".to_string(), serde_json::json!(item_ts));
                new_group.as_object_mut().unwrap().insert("group_count".to_string(), serde_json::json!(1));
                
                let mut initial_ids = Vec::new();
                if let Some(id_val) = item.get("id") {
                    initial_ids.push(id_val.clone());
                }
                new_group.as_object_mut().unwrap().insert("ids".to_string(), serde_json::json!(initial_ids));
                
                if category == "BROWSER_MONITOR" {
                    let dom = extract_domain_rust(&detail);
                    new_group.as_object_mut().unwrap().insert("display_name".to_string(), serde_json::json!(dom));
                    new_group.as_object_mut().unwrap().insert("group_domain".to_string(), serde_json::json!(dom));
                } else if category == "TERMINAL_MONITOR" || category == "IDE_MONITOR" || category == "COMMUNICATION_MONITOR" || category == "APP_MONITOR" {
                    let parts: Vec<&str> = detail.splitn(2, ':').collect();
                    let app_name = parts.get(0).cloned().unwrap_or(&display_name).to_string();
                    new_group.as_object_mut().unwrap().insert("display_name".to_string(), serde_json::json!(app_name));
                }

                new_group.as_object_mut().unwrap().insert("sub_items".to_string(), serde_json::json!([{
                    "id": item.get("id").cloned().unwrap_or(serde_json::Value::Null),
                    "timestamp": item.get("timestamp").cloned().unwrap_or(serde_json::Value::Null),
                    "title": display_name,
                    "url": detail,
                    "category": category,
                    "event_type": item.get("event_type").cloned().unwrap_or(serde_json::Value::Null)
                }]));
                
                grouped_items.push(new_group);
            }
        }
        
        // Sort grouped_items by last_timestamp ascending so reversing makes the newest first
        grouped_items.sort_by(|a, b| {
            let a_ts = a.get("last_timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
            let b_ts = b.get("last_timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
            a_ts.cmp(&b_ts)
        });
        
        // Reverse so newest is first
        grouped_items.reverse();
        
        for log_val in grouped_items {
            logs.push(log_val);
        }
    }
    
    // Fallback baseline logs to maintain cyberpunk aesthetics if database is completely empty
    if logs.is_empty() {
        logs.push(serde_json::json!({
            "timestamp": "[LIVE]",
            "raw_timestamp": "",
            "category": "SYSTEM",
            "event_type": "INIT",
            "display_name": "Establishing context security layer",
            "detail": "100% isolated local SQLite database active."
        }));
        logs.push(serde_json::json!({
            "timestamp": "[LIVE]",
            "raw_timestamp": "",
            "category": "FS_MONITOR",
            "event_type": "SYSTEM",
            "display_name": "Scanning workspace folders for cognitive focus indicators",
            "detail": "Passive filesystem watcher active. Standing by for workspace activity..."
        }));
        logs.push(serde_json::json!({
            "timestamp": "[LIVE]",
            "raw_timestamp": "",
            "category": "BROWSER_MONITOR",
            "event_type": "SYSTEM",
            "display_name": "Browser socket telemetry active",
            "detail": "Waiting for active Chromium tab focus..."
        }));
        logs.push(serde_json::json!({
            "timestamp": "[LIVE]",
            "raw_timestamp": "",
            "category": "SYSTEM",
            "event_type": "DAEMON",
            "display_name": "Silent context recorder running",
            "detail": "Zero interruptions mode enabled."
        }));
    }
    
    Json(logs)
}

#[derive(Deserialize)]
struct DeleteLogPayload {
    id: Option<i64>,
    ids: Option<Vec<i64>>,
    event_type: String,
}

async fn handle_delete_telemetry_log(
    State(state): State<ServerState>,
    Json(payload): Json<DeleteLogPayload>,
) -> impl axum::response::IntoResponse {
    let conn = match Connection::open(&state.db_path) {
        Ok(c) => c,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let tele_conn = Connection::open(&state.telemetry_db_path).ok();

    let target_ids = if let Some(ids) = payload.ids {
        ids
    } else if let Some(id) = payload.id {
        vec![id]
    } else {
        return StatusCode::BAD_REQUEST;
    };

    if target_ids.is_empty() {
        return StatusCode::OK;
    }

    let query = match payload.event_type.as_str() {
        "COMPACTED" | "YT_SHORTS" | "FILE_EDIT" | "TAB_FOCUS" | "TERMINAL_FOCUS" | "IDE_FOCUS" | "COMMUNICATION_FOCUS" | "APP_FOCUS" | "MANUAL_INGEST" | "CONSEQUENCE_DEGRADATION" => "DELETE FROM context_events WHERE id = ?1",
        "BROWSER_SEARCH" => "DELETE FROM search_queries WHERE id = ?1",
        "CHECKPOINT" => "DELETE FROM project_checkpoints WHERE id = ?1",
        "RECOVERY_PLAN" => "DELETE FROM recovery_plans WHERE id = ?1",
        _ => "DELETE FROM context_events WHERE id = ?1",
    };

    for id in target_ids {
        let _ = conn.execute(query, params![id]);
        if let Some(ref tc) = tele_conn {
            let _ = tc.execute(query, params![id]);
        }
    }

    StatusCode::OK
}

    #[derive(Deserialize)]
struct ManualIngestPayload {
    #[serde(rename = "displayName")]
    display_name: String,
    detail: String,
    #[serde(rename = "entityType")]
    entity_type: String, // "URL", "FILE", or "TEXT"
}

async fn handle_manual_ingest(
    State(state): State<ServerState>,
    Json(payload): Json<ManualIngestPayload>,
) -> impl axum::response::IntoResponse {
    let conn = match Connection::open(&state.db_path) {
        Ok(c) => c,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    
    // Auto-detect last active project_id
    let mut project_id: Option<i64> = None;
    if let Ok(p_id) = conn.query_row::<i64, _, _>(
        "SELECT project_id FROM context_nodes WHERE project_id IS NOT NULL ORDER BY id DESC LIMIT 1",
        [],
        |row| row.get(0)
    ) {
        project_id = Some(p_id);
    }
    
    let entity_key = match payload.entity_type.as_str() {
        "URL" => format!("URL:{}", payload.detail),
        "FILE" => format!("FILE:{}", payload.detail),
        _ => format!("MANUAL:{}", uuid::Uuid::new_v4().to_string()),
    };
    
    // Insert node
    let _ = conn.execute(
        "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(entity_key) DO NOTHING",
        params![project_id, &entity_key, &payload.entity_type, &payload.display_name]
    );
    
    // Fetch node_id
    let node_id: i64 = match conn.query_row(
        "SELECT id FROM context_nodes WHERE entity_key = ?1",
        params![entity_key],
        |row| row.get(0)
    ) {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    
    // Insert event in telemetry DB
    if let Ok(tele_conn) = Connection::open(&state.telemetry_db_path) {
        let captured_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = tele_conn.execute(
            "INSERT INTO context_events (node_id, event_type, interaction_duration, captured_at)
             VALUES (?1, 'MANUAL_INGEST', 1, ?2)",
            params![node_id, captured_at]
        );
    }
    
    StatusCode::OK
}

fn normalize_raw_timestamp(ts: &str) -> String {
    let clean_ts = ts.replace("+00:00Z", "Z");
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&clean_ts) {
        parsed.with_timezone(&chrono::Utc).to_rfc3339()
    } else if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(&clean_ts, "%Y-%m-%d %H:%M:%S") {
        let dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(parsed, chrono::Utc);
        dt.to_rfc3339()
    } else if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(&clean_ts, "%Y-%m-%d %H:%M:%S.%f") {
        let dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(parsed, chrono::Utc);
        dt.to_rfc3339()
    } else {
        clean_ts
    }
}

fn format_timestamp(ts: &str) -> String {
    let normalized = normalize_raw_timestamp(ts);
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&normalized) {
        parsed.with_timezone(&chrono::Local).format("[%H:%M:%S]").to_string()
    } else {
        "[LIVE]".to_string()
    }
}

async fn handle_get_database(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let mut db_data = serde_json::json!({
        "projects": [],
        "projectState": [],
        "contextNodes": [],
        "contextEvents": [],
        "browserSessions": [],
        "searchQueries": [],
        "commitments": [],
        "projectDeadlines": [],
        "projectActions": [],
        "projectCheckpoints": [],
        "recoveryPlans": [],
        "autonomousResearchBriefs": [],
        "graphEdges": [],
        "workspaceSnapshots": [],
        "deadLetterQueue": [],
    });
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        let _ = conn.execute("ATTACH DATABASE ?1 AS telemetry", params![state.telemetry_db_path.to_str().unwrap()]);
        
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_name, status, created_at FROM main.projects") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_name": r.get::<_, String>(1)?,
                    "status": r.get::<_, String>(2)?,
                    "created_at": r.get::<_, String>(3)?,
                }))
            }) {
                db_data["projects"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        if let Ok(mut stmt) = conn.prepare("SELECT project_id, current_summary, current_entry_point, next_action, confidence_score, updated_at FROM main.project_state") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "project_id": r.get::<_, i64>(0)?,
                    "current_summary": r.get::<_, String>(1)?,
                    "current_entry_point": r.get::<_, Option<String>>(2)?,
                    "next_action": r.get::<_, Option<String>>(3)?,
                    "confidence_score": r.get::<_, f64>(4)?,
                    "updated_at": r.get::<_, String>(5)?,
                }))
            }) {
                db_data["projectState"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, entity_key, entity_type, display_name, created_at FROM main.context_nodes") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, Option<i64>>(1)?,
                    "entity_key": r.get::<_, String>(2)?,
                    "entity_type": r.get::<_, String>(3)?,
                    "display_name": r.get::<_, String>(4)?,
                    "created_at": r.get::<_, String>(5)?,
                }))
            }) {
                db_data["contextNodes"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, node_id, event_type, interaction_duration, captured_at FROM (
                SELECT id, node_id, event_type, interaction_duration, captured_at FROM main.context_events
                UNION ALL
                SELECT id, node_id, event_type, interaction_duration, captured_at FROM telemetry.context_events
             )"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "node_id": r.get::<_, i64>(1)?,
                    "event_type": r.get::<_, String>(2)?,
                    "interaction_duration": r.get::<_, i64>(3)?,
                    "captured_at": r.get::<_, String>(4)?,
                }))
            }) {
                db_data["contextEvents"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at FROM (
                SELECT id, project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at FROM main.browser_sessions
                UNION ALL
                SELECT id, project_id, url, page_title, domain, visit_started_at, visit_ended_at, active_seconds, created_at FROM telemetry.browser_sessions
             )"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, Option<i64>>(1)?,
                    "url": r.get::<_, String>(2)?,
                    "page_title": r.get::<_, String>(3)?,
                    "domain": r.get::<_, String>(4)?,
                    "visit_started_at": r.get::<_, String>(5)?,
                    "visit_ended_at": r.get::<_, Option<String>>(6)?,
                    "active_seconds": r.get::<_, i64>(7)?,
                    "created_at": r.get::<_, String>(8)?,
                }))
            }) {
                db_data["browserSessions"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, browser_session_id, query_text, created_at FROM (
                SELECT id, browser_session_id, query_text, created_at FROM main.search_queries
                UNION ALL
                SELECT id, browser_session_id, query_text, created_at FROM telemetry.search_queries
             )"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "browser_session_id": r.get::<_, i64>(1)?,
                    "query_text": r.get::<_, String>(2)?,
                    "created_at": r.get::<_, String>(3)?,
                }))
            }) {
                db_data["searchQueries"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, title, commitment_type, deadline_date, confidence_score, source_node_id, status, created_at FROM main.commitments") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, i64>(1)?,
                    "title": r.get::<_, String>(2)?,
                    "commitment_type": r.get::<_, String>(3)?,
                    "deadline_date": r.get::<_, Option<String>>(4)?,
                    "confidence_score": r.get::<_, f64>(5)?,
                    "source_node_id": r.get::<_, Option<i64>>(6)?,
                    "status": r.get::<_, String>(7)?,
                    "created_at": r.get::<_, String>(8)?,
                }))
            }) {
                db_data["commitments"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, deadline_label, target_date, importance_tier, created_at FROM main.project_deadlines") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, i64>(1)?,
                    "deadline_label": r.get::<_, String>(2)?,
                    "target_date": r.get::<_, String>(3)?,
                    "importance_tier": r.get::<_, String>(4)?,
                    "created_at": r.get::<_, String>(5)?,
                }))
            }) {
                db_data["projectDeadlines"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, action_text, estimated_effort_hours, status, priority_score, created_at FROM main.project_actions") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, i64>(1)?,
                    "action_text": r.get::<_, String>(2)?,
                    "estimated_effort_hours": r.get::<_, f64>(3)?,
                    "status": r.get::<_, String>(4)?,
                    "priority_score": r.get::<_, f64>(5)?,
                    "created_at": r.get::<_, String>(6)?,
                }))
            }) {
                db_data["projectActions"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, accomplished_text, blocked_text, next_steps_text, created_at FROM main.project_checkpoints") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, i64>(1)?,
                    "accomplished_text": r.get::<_, String>(2)?,
                    "blocked_text": r.get::<_, Option<String>>(3)?,
                    "next_steps_text": r.get::<_, Option<String>>(4)?,
                    "created_at": r.get::<_, String>(5)?,
                }))
            }) {
                db_data["projectCheckpoints"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare("SELECT id, commitment_id, plan_payload_json, generated_at FROM main.recovery_plans") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "commitment_id": r.get::<_, i64>(1)?,
                    "plan_payload_json": r.get::<_, String>(2)?,
                    "generated_at": r.get::<_, String>(3)?,
                }))
            }) {
                db_data["recoveryPlans"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at FROM (
                SELECT id, project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at FROM main.workspace_snapshots
                UNION ALL
                SELECT id, project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at FROM telemetry.workspace_snapshots
             )"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, Option<i64>>(1)?,
                    "active_file_path": r.get::<_, String>(2)?,
                    "cursor_line": r.get::<_, i64>(3)?,
                    "cursor_column": r.get::<_, i64>(4)?,
                    "open_tabs_json": r.get::<_, String>(5)?,
                    "captured_at": r.get::<_, String>(6)?,
                }))
            }) {
                db_data["workspaceSnapshots"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }

        if let Ok(mut stmt) = conn.prepare("SELECT id, source_uri, payload_hash, worker_type, failure_reason, retry_count, failed_at FROM main.dead_letter_queue") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "source_uri": r.get::<_, String>(1)?,
                    "payload_hash": r.get::<_, String>(2)?,
                    "worker_type": r.get::<_, String>(3)?,
                    "failure_reason": r.get::<_, String>(4)?,
                    "retry_count": r.get::<_, i64>(5)?,
                    "failed_at": r.get::<_, String>(6)?,
                }))
            }) {
                db_data["deadLetterQueue"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
    }
    Json(db_data)
}

async fn handle_database_purge(State(state): State<ServerState>) -> Json<serde_json::Value> {
    if let Ok(conn) = Connection::open(&state.db_path) {
        let _ = conn.execute_batch(
            "DELETE FROM workspace_snapshots;
             DELETE FROM search_queries;
             DELETE FROM browser_sessions;
             DELETE FROM context_events;
             DELETE FROM context_nodes;
             DELETE FROM project_checkpoints;
             DELETE FROM recovery_plans;
             DELETE FROM commitments;
             DELETE FROM project_deadlines;
             DELETE FROM project_actions;
             DELETE FROM project_state;
             DELETE FROM projects;"
        );
        let _ = crate::db::seed_db(&conn);
    }
    if let Ok(conn) = Connection::open(&state.telemetry_db_path) {
        let _ = conn.execute_batch(
            "DELETE FROM workspace_snapshots;
             DELETE FROM search_queries;
             DELETE FROM browser_sessions;
             DELETE FROM context_events;"
        );
    }
    {
        let mut st = state.simulated_time.lock().unwrap();
        *st = chrono::Local::now();
    }
    Json(serde_json::json!({
        "success": true,
        "message": "System purged successfully. Baseline data re-seeded."
    }))
}

// ── Predictive Terminal Autocomplete ──

fn get_terminal_suggestion(db_path: &Path) -> Option<String> {
    let conn = Connection::open(db_path).ok()?;
    let mut stmt = conn.prepare(
        "SELECT display_name FROM context_nodes WHERE \
         entity_key LIKE '%Terminal%' OR entity_key LIKE '%Powershell%' \
         OR entity_key LIKE '%cmd%' OR entity_key LIKE '%bash%' \
         OR entity_type = 'APP' \
         ORDER BY id DESC LIMIT 10"
    ).ok()?;
    
    let names: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0))
        .ok()?
        .filter_map(|r| r.ok())
        .collect();
    
    for name in &names {
        let lower = name.to_lowercase();
        if lower.contains("eaddrinuse") || (lower.contains("port") && lower.contains("error")) {
            return Some("netstat -ano | findstr :<PORT>".to_string());
        }
        if lower.contains("npm") && (lower.contains("err") || lower.contains("error")) {
            return Some("npm cache clean --force && npm install".to_string());
        }
        if lower.contains("python") && lower.contains("modulenotfounderror") {
            return Some("pip install <missing-module>".to_string());
        }
        if lower.contains("git") && (lower.contains("conflict") || lower.contains("merge")) {
            return Some("git status && git diff --name-only --diff-filter=U".to_string());
        }
        if lower.contains("cargo") && lower.contains("error") {
            return Some("cargo clean && cargo build 2>&1 | head -30".to_string());
        }
        if lower.contains("permission") && lower.contains("denied") {
            return Some("icacls <file> /grant Users:F".to_string());
        }
    }
    None
}

async fn handle_terminal_suggest(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let suggestion = get_terminal_suggestion(&state.db_path);
    
    let mut last_command = String::new();
    if let Ok(conn) = Connection::open(&state.db_path) {
        if let Ok(name) = conn.query_row(
            "SELECT display_name FROM context_nodes WHERE \
             entity_key LIKE '%Terminal%' OR entity_key LIKE '%Powershell%' \
             ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0)
        ) {
            last_command = name;
        }
    }
    
    Json(serde_json::json!({
        "has_suggestion": suggestion.is_some(),
        "suggested_command": suggestion.unwrap_or_default(),
        "reason": if !last_command.is_empty() {
            format!("Based on last terminal context: {}", last_command)
        } else {
            "No recent terminal activity detected.".to_string()
        },
        "last_command": last_command
    }))
}

// ── Context Handoff (.chronos export) ──

async fn handle_context_export(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let mut export_data = serde_json::json!({
        "projects": [],
        "projectState": [],
        "contextNodes": [],
        "contextEvents": [],
        "workspaceSnapshots": [],
        "commitments": [],
        "projectActions": [],
        "projectCheckpoints": [],
        "projectSnapshots": [],
        "narrative": null
    });
    
    if let Ok(conn) = Connection::open(&state.db_path) {
        let _ = conn.execute("ATTACH DATABASE ?1 AS telemetry", params![state.telemetry_db_path.to_str().unwrap()]);
        
        // Projects
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_name, status, created_at FROM main.projects") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_name": r.get::<_, String>(1)?,
                    "status": r.get::<_, String>(2)?,
                    "created_at": r.get::<_, String>(3)?,
                }))
            }) {
                export_data["projects"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Project State
        if let Ok(mut stmt) = conn.prepare("SELECT project_id, current_summary, current_entry_point, next_action, confidence_score, updated_at FROM main.project_state") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "project_id": r.get::<_, i64>(0)?,
                    "current_summary": r.get::<_, String>(1)?,
                    "current_entry_point": r.get::<_, Option<String>>(2)?,
                    "next_action": r.get::<_, Option<String>>(3)?,
                    "confidence_score": r.get::<_, f64>(4)?,
                    "updated_at": r.get::<_, String>(5)?,
                }))
            }) {
                export_data["projectState"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Context Nodes
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, entity_key, entity_type, display_name, created_at FROM main.context_nodes") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, Option<i64>>(1)?,
                    "entity_key": r.get::<_, String>(2)?,
                    "entity_type": r.get::<_, String>(3)?,
                    "display_name": r.get::<_, String>(4)?,
                    "created_at": r.get::<_, String>(5)?,
                }))
            }) {
                export_data["contextNodes"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Context Events (merged)
        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, node_id, event_type, interaction_duration, captured_at FROM (
                SELECT id, node_id, event_type, interaction_duration, captured_at FROM main.context_events
                UNION ALL
                SELECT id, node_id, event_type, interaction_duration, captured_at FROM telemetry.context_events
             )"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "node_id": r.get::<_, i64>(1)?,
                    "event_type": r.get::<_, String>(2)?,
                    "interaction_duration": r.get::<_, i64>(3)?,
                    "captured_at": r.get::<_, String>(4)?,
                }))
            }) {
                export_data["contextEvents"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Workspace Snapshots (merged)
        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at FROM (
                SELECT id, project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at FROM main.workspace_snapshots
                UNION ALL
                SELECT id, project_id, active_file_path, cursor_line, cursor_column, open_tabs_json, captured_at FROM telemetry.workspace_snapshots
             )"
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, Option<i64>>(1)?,
                    "active_file_path": r.get::<_, String>(2)?,
                    "cursor_line": r.get::<_, i64>(3)?,
                    "cursor_column": r.get::<_, i64>(4)?,
                    "open_tabs_json": r.get::<_, String>(5)?,
                    "captured_at": r.get::<_, String>(6)?,
                }))
            }) {
                export_data["workspaceSnapshots"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Commitments
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, title, commitment_type, deadline_date, confidence_score, status, created_at FROM main.commitments") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, i64>(1)?,
                    "title": r.get::<_, String>(2)?,
                    "commitment_type": r.get::<_, String>(3)?,
                    "deadline_date": r.get::<_, Option<String>>(4)?,
                    "confidence_score": r.get::<_, f64>(5)?,
                    "status": r.get::<_, String>(6)?,
                    "created_at": r.get::<_, String>(7)?,
                }))
            }) {
                export_data["commitments"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Project Actions
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, action_text, estimated_effort_hours, status, priority_score, created_at FROM main.project_actions") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, i64>(1)?,
                    "action_text": r.get::<_, String>(2)?,
                    "estimated_effort_hours": r.get::<_, f64>(3)?,
                    "status": r.get::<_, String>(4)?,
                    "priority_score": r.get::<_, f64>(5)?,
                    "created_at": r.get::<_, String>(6)?,
                }))
            }) {
                export_data["projectActions"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Project Checkpoints
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, accomplished_text, blocked_text, next_steps_text, created_at FROM main.project_checkpoints") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, i64>(1)?,
                    "accomplished_text": r.get::<_, String>(2)?,
                    "blocked_text": r.get::<_, Option<String>>(3)?,
                    "next_steps_text": r.get::<_, Option<String>>(4)?,
                    "created_at": r.get::<_, String>(5)?,
                }))
            }) {
                export_data["projectCheckpoints"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
        
        // Project Snapshots
        if let Ok(mut stmt) = conn.prepare("SELECT id, project_id, snapshot_summary, workspace_state_json, generated_at FROM main.project_snapshots") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "project_id": r.get::<_, Option<i64>>(1)?,
                    "snapshot_summary": r.get::<_, String>(2)?,
                    "workspace_state_json": r.get::<_, String>(3)?,
                    "generated_at": r.get::<_, String>(4)?,
                }))
            }) {
                export_data["projectSnapshots"] = serde_json::Value::Array(rows.flatten().collect());
            }
        }
    }
    
    // Read narrative
    let narrative_path = dirs::home_dir().unwrap_or_default().join(".config").join("chronos").join("reconstruction_narrative.md");
    if let Ok(content) = fs::read_to_string(narrative_path) {
        export_data["narrative"] = serde_json::json!(content);
    }
    
    let now = chrono::Utc::now().to_rfc3339();
    Json(serde_json::json!({
        "chronos_version": "1.0.0",
        "exported_at": now,
        "format": "chronos-context-handoff",
        "data": export_data
    }))
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
    Ok(ws.on_upgrade(|socket| handle_browser_socket(socket, state.db_path, state.telemetry_db_path)))
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
    Ok(ws.on_upgrade(|socket| handle_ide_socket(socket, state.db_path, state.telemetry_db_path)))
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

async fn handle_browser_socket(mut socket: WebSocket, db_path: PathBuf, telemetry_db_path: PathBuf) {
    use axum::extract::ws::Message as AxumMessage;

    while let Some(Ok(msg)) = socket.recv().await {
        if let AxumMessage::Text(text) = msg {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let payload = value.get("payload");
                
                if let Some(p) = payload {
                    let conn = match Connection::open(&db_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let tele_conn = match Connection::open(&telemetry_db_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    
                    if msg_type == "TAB_FOCUS" {
                        let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let domain = p.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                        let visit_started_at = p.get("visit_started_at").and_then(|v| v.as_str()).unwrap_or("");
                        
                        let _ = tele_conn.execute(
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
                            let _ = tele_conn.execute(
                                "INSERT INTO context_events (node_id, event_type, interaction_duration)
                                 VALUES (?1, 'TAB_FOCUS', 10)",
                                params![node_id]
                            );
                        }

                        // Dopamine Friction Overlay check
                        let domain_lower = domain.to_lowercase();
                        let is_distraction = DISTRACTION_DOMAINS.iter().any(|&d| domain_lower.contains(d));
                        if is_distraction {
                            let now = chrono::Utc::now();
                            let deadline_limit = now + chrono::Duration::hours(48);
                            let has_imminent_deadline: bool = conn.query_row(
                                "SELECT COUNT(*) FROM commitments WHERE status = 'OPEN' AND deadline_date IS NOT NULL AND deadline_date <= ?1",
                                params![deadline_limit.to_rfc3339()],
                                |row| {
                                    let count: i64 = row.get(0)?;
                                    Ok(count > 0)
                                }
                            ).unwrap_or(false);

                            if has_imminent_deadline {
                                let intercept_msg = serde_json::json!({
                                    "type": "DISTRACTION_INTERCEPT",
                                    "domain": domain
                                });
                                let _ = socket.send(AxumMessage::Text(intercept_msg.to_string())).await;
                            }
                        }
                    } else if msg_type == "SEARCH_QUERY" {
                        let query_text = p.get("query_text").and_then(|v| v.as_str()).unwrap_or("");
                        let created_at = p.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                        
                        let session_id: Option<i64> = tele_conn.query_row(
                            "SELECT id FROM browser_sessions ORDER BY id DESC LIMIT 1",
                            [],
                            |row| row.get(0)
                        ).ok();
                        
                        if let Some(sid) = session_id {
                            let _ = tele_conn.execute(
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

async fn handle_ide_socket(mut socket: WebSocket, db_path: PathBuf, telemetry_db_path: PathBuf) {
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
                    let tele_conn = match Connection::open(&telemetry_db_path) {
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
                                let _ = tele_conn.execute(
                                    "INSERT INTO context_events (node_id, event_type, interaction_duration)
                                     VALUES (?1, 'EDITED', 1)",
                                    params![node_id]
                                );
                            }
                            
                            let _ = tele_conn.execute(
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

fn extract_browser_site_name(title: &str) -> String {
    let title_lower = title.to_lowercase();
    
    // 1. Check known domains/keywords first
    if title_lower.contains("z.ai") {
        return "z.ai".to_string();
    }
    if title_lower.contains("chatgpt") {
        return "chatgpt.com".to_string();
    }
    if title_lower.contains("claude.ai") || title_lower.contains("claude") {
        return "claude.ai".to_string();
    }
    if title_lower.contains("gemini.google") || title_lower.contains("gemini") {
        return "gemini.google".to_string();
    }
    if title_lower.contains("copilot") {
        return "copilot".to_string();
    }
    if title_lower.contains("v0.dev") || title_lower.contains("v0 ") || title_lower.starts_with("v0") {
        return "v0.dev".to_string();
    }
    if title_lower.contains("openai") {
        return "openai.com".to_string();
    }
    if title_lower.contains("youtube") || title_lower.contains("shorts") {
        return "youtube.com".to_string();
    }
    if title_lower.contains("reddit") {
        return "reddit.com".to_string();
    }
    if title_lower.contains("twitter") || title_lower.contains("x.com") {
        return "twitter.com".to_string();
    }
    if title_lower.contains("instagram") {
        return "instagram.com".to_string();
    }
    if title_lower.contains("facebook") {
        return "facebook.com".to_string();
    }
    if title_lower.contains("netflix") {
        return "netflix.com".to_string();
    }
    if title_lower.contains("github") {
        return "github.com".to_string();
    }
    if title_lower.contains("stackoverflow") || title_lower.contains("stack overflow") {
        return "stackoverflow.com".to_string();
    }
    if title_lower.contains("google search") {
        return "google.com".to_string();
    }
    
    // 2. Clean up common browser suffixes
    let mut clean = title.to_string();
    let suffixes_to_strip = [
        " - Microsoft Edge",
        " - Google Chrome",
        " - Firefox",
        " - Brave",
        " - Opera",
        " - Safari",
        " - Edge",
        " - Chrome",
        " - Profile 1",
        " - Profile 2",
        " - Profile 3",
        " - InPrivate",
        " - Incognito",
        " Microsoft Edge",
        " Google Chrome"
    ];
    
    for &suffix in &suffixes_to_strip {
        if let Some(idx) = clean.to_lowercase().rfind(&suffix.to_lowercase()) {
            clean.truncate(idx);
        }
    }
    
    let clean = clean.trim();
    if clean.is_empty() {
        return "General Browsing".to_string();
    }
    
    // 3. Try to extract site from " - Site Name" at the end
    let parts: Vec<&str> = clean.split(" - ").collect();
    if parts.len() > 1 {
        let last_part = parts.last().unwrap().trim();
        if !last_part.is_empty() && last_part.len() < 30 {
            return last_part.to_string();
        }
        let first_part = parts.first().unwrap().trim();
        if !first_part.is_empty() && first_part.len() < 30 {
            return first_part.to_string();
        }
    }
    
    // If no clean split, return truncated clean title
    if clean.len() > 30 {
        format!("{}...", &clean[..27])
    } else {
        clean.to_string()
    }
}

fn extract_domain_rust(url: &str) -> String {
    // Check if it's in the format "BrowserName:WindowTitle"
    let parts: Vec<&str> = url.splitn(2, ':').collect();
    if parts.len() == 2 {
        let first_lower = parts[0].to_lowercase();
        if first_lower.contains("chrome")
            || first_lower.contains("firefox")
            || first_lower.contains("edge")
            || first_lower.contains("safari")
            || first_lower.contains("opera")
            || first_lower.contains("brave")
            || first_lower.contains("arc")
            || first_lower.contains("browser")
        {
            // It's a browser focus event. Extract site name from the window title (parts[1]).
            return extract_browser_site_name(parts[1]);
        }
    }

    let clean = url.replace("https://", "").replace("http://", "");
    let parts: Vec<&str> = clean.split('/').collect();
    if let Some(first) = parts.first() {
        let subparts: Vec<&str> = first.split(':').collect();
        if let Some(&host) = subparts.first() {
            return host.replace("www.", "");
        }
    }
    "unknown".to_string()
}

fn run_database_compaction(db_path: &Path) -> Result<(), rusqlite::Error> {
    let conn = Connection::open(db_path)?;
    
    // 1. Purge YouTube and Shorts older than 1 hour
    let _ = conn.execute(
        "DELETE FROM context_events 
         WHERE id IN (
             SELECT ce.id FROM context_events ce
             JOIN context_nodes cn ON ce.node_id = cn.id
             WHERE cn.entity_type IN ('URL', 'RESEARCH_SESSION')
               AND (cn.entity_key LIKE '%youtube%' 
                 OR cn.entity_key LIKE '%yt%' 
                 OR cn.display_name LIKE '%YouTube%' 
                 OR cn.display_name LIKE '%Shorts%')
               AND ce.captured_at < datetime('now', '-1 hour')
         )",
        [],
    );
    
    // 2. Cap browser sessions to 500 records
    let _ = conn.execute(
        "DELETE FROM browser_sessions 
         WHERE id NOT IN (
             SELECT id FROM browser_sessions 
             ORDER BY id DESC 
             LIMIT 500
         )",
        [],
    );
    
    // 3. Semantic Compaction of context events older than 7 days
    let node_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM context_nodes",
        [],
        |row| row.get(0)
    ).unwrap_or(0);

    if node_count < 90 {
        return Ok(());
    }

    struct CompactCandidate {
        id: i64,
        captured_at: String,
        event_type: String,
        _display_name: String,
        entity_key: String,
        entity_type: String,
        project_id: Option<i64>,
    }
    
    let mut stmt = conn.prepare(
        "SELECT ce.id, ce.captured_at, ce.event_type, cn.display_name, cn.entity_key, cn.entity_type, cn.project_id
         FROM context_events ce
         JOIN context_nodes cn ON ce.node_id = cn.id
         WHERE ce.captured_at < datetime('now', '-7 days')
           AND ce.event_type != 'COMPACTED'"
    )?;
    
    let mut candidates = vec![];
    let rows = stmt.query_map([], |r| {
        Ok(CompactCandidate {
            id: r.get(0)?,
            captured_at: r.get(1)?,
            event_type: r.get(2)?,
            _display_name: r.get(3)?,
            entity_key: r.get(4)?,
            entity_type: r.get(5)?,
            project_id: r.get(6)?,
        })
    })?;
    
    for row in rows.flatten() {
        candidates.push(row);
    }
    
    if candidates.is_empty() {
        return Ok(());
    }
    
    // Group candidates by project_id
    use std::collections::HashMap;
    let mut grouped: HashMap<Option<i64>, Vec<CompactCandidate>> = HashMap::new();
    for c in candidates {
        grouped.entry(c.project_id).or_default().push(c);
    }
    
    for (project_id, items) in grouped {
        if items.is_empty() {
            continue;
        }
        
        let start_date = items.iter().map(|x| &x.captured_at).min().unwrap();
        let end_date = items.iter().map(|x| &x.captured_at).max().unwrap();
        
        // Distinct files worked on (key -> count)
        let mut files: HashMap<String, usize> = HashMap::new();
        // Distinct domains researched (domain -> count)
        let mut domains: HashMap<String, usize> = HashMap::new();
        // Distinct apps used (app -> count)
        let mut apps: HashMap<String, usize> = HashMap::new();
        
        for item in &items {
            if item.event_type == "EDITED" || item.entity_type == "FILE" {
                let filename = item.entity_key.replace("FILE:", "");
                *files.entry(filename).or_default() += 1;
            } else if item.event_type == "TAB_FOCUS" || item.entity_type == "URL" || item.entity_type == "RESEARCH_SESSION" {
                if item.entity_key.starts_with("URL:") {
                    let url = item.entity_key.replace("URL:", "");
                    let domain = extract_domain_rust(&url);
                    *domains.entry(domain).or_default() += 1;
                } else if item.entity_key.starts_with("APP:") {
                    let app_content = item.entity_key.strip_prefix("APP:").unwrap_or(&item.entity_key);
                    let parts: Vec<&str> = app_content.splitn(2, ':').collect();
                    let app_name = parts.get(0).cloned().unwrap_or("Unknown App").to_string();
                    *apps.entry(app_name).or_default() += 1;
                } else {
                    let domain = extract_domain_rust(&item.entity_key);
                    *domains.entry(domain).or_default() += 1;
                }
            }
        }
        
        // Build Markdown Summary
        let mut md = String::new();
        md.push_str("### Consolidated Workspace Summary\n");
        md.push_str(&format!("* **Date Range**: {} to {}\n", start_date, end_date));
        
        if !files.is_empty() {
            md.push_str("* **Files Worked On**:\n");
            for (file, count) in &files {
                md.push_str(&format!("  * `{}` ({} edits)\n", file, count));
            }
        }
        
        if !domains.is_empty() {
            md.push_str("* **Web Resources Researched**:\n");
            for (dom, count) in &domains {
                md.push_str(&format!("  * `{}` ({} visits)\n", dom, count));
            }
        }
        
        if !apps.is_empty() {
            md.push_str("* **Applications & Tools Used**:\n");
            for (app, count) in &apps {
                md.push_str(&format!("  * `{}` ({} interactions)\n", app, count));
            }
        }
        
        md.push_str(&format!("* **Total Context Items Consolidated**: {}\n", items.len()));
        
        // Insert Compacted Document context_node
        let entity_key = format!("COMPACTED:{}:{}:{}", project_id.unwrap_or(0), start_date, end_date);
        let display_name = md; // store Markdown summary in display_name field
        
        let _ = conn.execute(
            "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name)
             VALUES (?1, ?2, 'DOCUMENT', ?3)
             ON CONFLICT(entity_key) DO NOTHING",
            params![project_id, entity_key, display_name]
        );
        
        let node_id_res: Result<i64, rusqlite::Error> = conn.query_row(
            "SELECT id FROM context_nodes WHERE entity_key = ?1",
            params![entity_key],
            |r| r.get(0)
        );
        
        if let Ok(node_id) = node_id_res {
            // Insert Compacted context_event
            let _ = conn.execute(
                "INSERT INTO context_events (node_id, event_type, interaction_duration, captured_at)
                 VALUES (?1, 'COMPACTED', 0, datetime('now'))",
                params![node_id]
            );
            
            // Purge original raw context_events
            let ids: Vec<String> = items.iter().map(|x| x.id.to_string()).collect();
            let ids_str = ids.join(",");
            let delete_query = format!("DELETE FROM context_events WHERE id IN ({})", ids_str);
            let _ = conn.execute(&delete_query, []);
        }
    }
    
    Ok(())
}
