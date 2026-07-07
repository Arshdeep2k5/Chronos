#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod db;
mod watcher;
mod window_focus;
mod server;
mod che;
mod consequence;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::thread;
use std::process::{Command, Child};

pub fn spawn_python_worker(auth_token: &str, port: u16) -> Option<Child> {
    let worker_script = Path::new("D:\\Chronos_Hackathon\\python-worker\\worker.py");
    
    let python_cmd = if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    };
    
    match Command::new(python_cmd)
        .arg(worker_script)
        .arg("--token")
        .arg(auth_token)
        .arg("--port")
        .arg(port.to_string())
        .spawn() 
    {
        Ok(child) => {
            println!("Spawned Python worker with PID: {}", child.id());
            Some(child)
        }
        Err(e) => {
            println!("Failed to spawn Python worker: {:?}", e);
            None
        }
    }
}

pub fn spawn_simulator_worker(port: u16) -> Option<Child> {
    let worker_script = Path::new("D:\\Chronos_Hackathon\\python-worker\\simulator_worker.py");
    
    let python_cmd = if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    };
    
    match Command::new(python_cmd)
        .arg(worker_script)
        .arg("--port")
        .arg(port.to_string())
        .spawn() 
    {
        Ok(child) => {
            println!("Spawned Python Simulator worker with PID: {}", child.id());
            Some(child)
        }
        Err(e) => {
            println!("Failed to spawn Python Simulator worker: {:?}", e);
            None
        }
    }
}

pub fn start_heartbeat_monitor(
    auth_token: String,
    port: u16,
    last_heartbeat: Arc<Mutex<Instant>>,
) {
    thread::spawn(move || {
        let mut child = spawn_python_worker(&auth_token, port);
        let mut restart_times = Vec::new();
        let mut failed_state = false;
        
        loop {
            thread::sleep(Duration::from_secs(5));
            
            if failed_state {
                continue;
            }
            
            let is_unresponsive = {
                let lh = last_heartbeat.lock().unwrap();
                lh.elapsed() > Duration::from_secs(15)
            };
            
            let mut is_dead = false;
            if let Some(ref mut c) = child {
                match c.try_wait() {
                    Ok(Some(status)) => {
                        println!("Python worker exited with status: {}", status);
                        is_dead = true;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        println!("Error checking python worker state: {:?}", e);
                    }
                }
            } else {
                is_dead = true;
            }
            
            if is_unresponsive || is_dead {
                println!("Python worker unresponsive or dead. Restarting...");
                
                if let Some(ref mut c) = child {
                    let _ = c.kill();
                }
                
                let now = Instant::now();
                restart_times.retain(|&t: &Instant| now.duration_since(t) < Duration::from_secs(600));
                
                if restart_times.len() >= 3 {
                    println!("CRITICAL: Python worker crashed too many times. Entering FAILED_STATE.");
                    failed_state = true;
                    continue;
                }
                
                restart_times.push(now);
                {
                    let mut lh = last_heartbeat.lock().unwrap();
                    *lh = Instant::now();
                }
                child = spawn_python_worker(&auth_token, port);
            }
        }
    });
}

fn load_env_file() {
    let paths = [
        "D:\\Chronos_Hackathon\\chronos-pilot\\.env",
        "D:\\Chronos_Hackathon\\chronos-pilot\\.env.example",
        "D:\\Chronos_Hackathon\\.env",
        "D:\\Chronos_Hackathon\\.env.example",
    ];
    
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            println!("Reading env from {:?}", path);
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, val)) = line.split_once('=') {
                    let key = key.trim();
                    let val = val.trim();
                    
                    let mut val_str = val.to_string();
                    if (val_str.starts_with('"') && val_str.ends_with('"')) ||
                       (val_str.starts_with('\'') && val_str.ends_with('\'')) {
                        if val_str.len() >= 2 {
                            val_str.remove(0);
                            val_str.pop();
                        }
                    }
                    
                    if key == "GEMINI_API_KEY" && !val_str.is_empty() {
                        println!("Loaded GEMINI_API_KEY from {:?}", path);
                        std::env::set_var("GEMINI_API_KEY", &val_str);
                        return;
                    }
                }
            }
        }
    }
}

fn main() {
    load_env_file();
    
    let home_dir = std::env::var("USERPROFILE")
        .unwrap_or_else(|_| std::env::var("HOME")
        .unwrap_or_else(|_| ".".to_string()));
    
    let config_dir = PathBuf::from(home_dir).join(".config").join("chronos");
    let db_path = config_dir.join("chronos.db");
    let telemetry_db_path = config_dir.join("chronos_telemetry.db");
    
    println!("Database path: {:?}", db_path);
    println!("Telemetry database path: {:?}", telemetry_db_path);
    
    // Initialize DBs
    let _conn = db::init_db(&db_path).expect("Failed to initialize database");
    let _telemetry_conn = db::init_telemetry_db(&telemetry_db_path).expect("Failed to initialize telemetry database");
    
    // Setup server and token
    let temp_auth_token = uuid::Uuid::new_v4().simple().to_string();
    let last_heartbeat = Arc::new(Mutex::new(Instant::now()));
    
    // Build server state and start server
    let db_path_clone = db_path.clone();
    let telemetry_db_path_clone = telemetry_db_path.clone();
    let auth_token_clone = temp_auth_token.clone();
    let last_hb_clone = last_heartbeat.clone();
    
    // Create a tokio runtime to run our server
    let rt = tokio::runtime::Runtime::new().unwrap();
    let port = rt.block_on(async {
        server::start_server(db_path_clone, telemetry_db_path_clone, auth_token_clone, last_hb_clone).await
    });
    
    // Write the handshake json with active port and correct token
    let _handshake = server::generate_handshake(&config_dir, temp_auth_token.clone(), port);
    
    // Spawn Python subprocess and start monitoring
    let _sim_child = spawn_simulator_worker(48130);
    start_heartbeat_monitor(temp_auth_token, port, last_heartbeat);
    
    // Start FS File Ingestion Watcher
    let paths_to_watch = vec![
        PathBuf::from(config_dir.clone()),
        PathBuf::from(format!("{}\\Downloads", std::env::var("USERPROFILE").unwrap_or_default())),
    ];
    watcher::start_watcher(db_path, telemetry_db_path.clone(), paths_to_watch);
    
    // Start window focus loop
    window_focus::start_window_focus_loop(config_dir.join("chronos.db"), config_dir.join("chronos_telemetry.db"));
    
    // Launch Tauri app framework
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
