use notify::{Watcher, RecursiveMode, Event, RecommendedWatcher, Config};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::thread;
use rusqlite::{params, Connection};

pub fn start_watcher(db_path: PathBuf, paths_to_watch: Vec<PathBuf>) {
    thread::spawn(move || {
        let (tx, rx) = channel();
        
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                println!("Failed to create watcher: {:?}", e);
                return;
            }
        };
        
        for path in &paths_to_watch {
            if path.exists() {
                let _ = watcher.watch(path, RecursiveMode::Recursive);
            }
        }
        
        for res in rx {
            match res {
                Ok(event) => {
                    handle_watcher_event(&db_path, event);
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });
}

fn handle_watcher_event(db_path: &Path, event: Event) {
    if event.kind.is_create() || event.kind.is_modify() {
        for path in event.paths {
            if path.is_dir() {
                continue;
            }
            
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            let is_doc = matches!(ext.as_str(), "pdf" | "docx" | "doc" | "md" | "txt");
            let is_code = matches!(ext.as_str(), "ts" | "js" | "py" | "rs" | "go" | "cpp" | "c" | "java" | "json" | "html" | "css");
            
            if !is_doc && !is_code {
                continue;
            }
            
            let entity_type = if is_doc { "DOCUMENT" } else { "FILE" };
            let entity_key = format!("{}:{}", entity_type, path.to_string_lossy());
            let display_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string();
            
            if let Ok(conn) = Connection::open(db_path) {
                let _ = conn.execute(
                    "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name)
                     VALUES (NULL, ?1, ?2, ?3)
                     ON CONFLICT(entity_key) DO UPDATE SET created_at = datetime('now')",
                    params![entity_key, entity_type, display_name]
                );
                
                if let Ok(node_id) = conn.query_row::<i64, _, _>(
                    "SELECT id FROM context_nodes WHERE entity_key = ?1",
                    params![entity_key],
                    |row| row.get(0)
                ) {
                    let event_type = if event.kind.is_create() { "CREATED" } else { "EDITED" };
                    let _ = conn.execute(
                        "INSERT INTO context_events (node_id, event_type, interaction_duration)
                         VALUES (?1, ?2, 0)",
                        params![node_id, event_type]
                    );
                }
            }
        }
    }
}
