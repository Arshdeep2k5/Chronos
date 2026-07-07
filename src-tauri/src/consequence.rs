use rusqlite::{Connection, params};
use chrono::DateTime;
use chrono::Local;

pub async fn run_consequence_engine(
    conn: &mut Connection, 
    results: Vec<crate::che::SimulatorResult>,
    simulated_time: DateTime<Local>
) {
    for res in results {
        if res.p_success < 0.45 {
            let project_id: Option<i64> = conn.query_row(
                "SELECT project_id FROM commitments WHERE id = ?",
                [res.id],
                |row| row.get(0)
            ).ok().flatten();

            if let Some(pid) = project_id {
                let this_deadline: Option<String> = conn.query_row(
                    "SELECT deadline_date FROM commitments WHERE id = ?",
                    [res.id],
                    |row| row.get(0)
                ).ok().flatten();

                if let Some(td) = this_deadline {
                    let mut stmt = conn.prepare(
                        "SELECT id FROM commitments WHERE project_id = ? AND deadline_date > ? AND status = 'OPEN'"
                    ).unwrap();

                    let rows = stmt.query_map(params![pid, td], |row| row.get::<_, i64>(0)).unwrap();
                    for r in rows.flatten() {
                        let downstream_id = r;
                        let _ = conn.execute(
                            "UPDATE commitments SET confidence_score = confidence_score * 0.9 WHERE id = ?",
                            [downstream_id]
                        );
                        
                        let _ = conn.execute(
                            "INSERT INTO context_events (node_id, event_type, interaction_duration, captured_at)
                             VALUES (?, 'CONSEQUENCE_DEGRADATION', 0, ?)",
                            params![downstream_id, simulated_time.to_rfc3339()]
                        );
                    }
                }
            }
        }
    }
}
