use active_win_pos_rs::get_active_window;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use rusqlite::{params, Connection};

pub fn start_window_focus_loop(db_path: PathBuf) {
    thread::spawn(move || {
        let mut last_window_title = String::new();
        let mut last_app_name = String::new();
        
        loop {
            thread::sleep(Duration::from_secs(1));
            
            if let Ok(active_window) = get_active_window() {
                let app_name = active_window.app_name;
                let title = active_window.title;
                
                if app_name != last_app_name || title != last_window_title {
                    last_app_name = app_name.clone();
                    last_window_title = title.clone();
                    
                    log_window_focus(&db_path, &app_name, &title);
                }
            }
        }
    });
}

fn log_window_focus(db_path: &PathBuf, app_name: &str, title: &str) {
    let entity_key = format!("APP:{}:{}", app_name, title);
    let display_name = format!("{} - {}", app_name, title);
    
    if let Ok(conn) = Connection::open(db_path) {
        let _ = conn.execute(
            "INSERT INTO context_nodes (project_id, entity_key, entity_type, display_name)
             VALUES (NULL, ?1, 'RESEARCH_SESSION', ?2)
             ON CONFLICT(entity_key) DO NOTHING",
            params![entity_key, display_name]
        );
        
        if let Ok(node_id) = conn.query_row::<i64, _, _>(
            "SELECT id FROM context_nodes WHERE entity_key = ?1",
            params![entity_key],
            |row| row.get(0)
        ) {
            let _ = conn.execute(
                "INSERT INTO context_events (node_id, event_type, interaction_duration)
                 VALUES (?1, 'TAB_FOCUS', 1)",
                params![node_id]
            );
        }
    }
}
