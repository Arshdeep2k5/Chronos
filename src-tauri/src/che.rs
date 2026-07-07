use rusqlite::Connection;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SimulatorRequest {
    pub commitments: Vec<CommitmentPayload>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitmentPayload {
    pub id: i64,
    pub time_remaining_hours: f64,
    pub estimated_effort_hours: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimulatorResponse {
    pub results: Vec<SimulatorResult>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimulatorResult {
    pub id: i64,
    pub p_success: f64,
}

pub async fn run_che_loop(
    db_path: std::path::PathBuf,
    simulated_time: Arc<Mutex<chrono::DateTime<chrono::Local>>>,
    simulator_port: u16,
) {
    let client = reqwest::Client::new();
    loop {
        sleep(Duration::from_secs(300)).await;

        let st = {
            let guard = simulated_time.lock().unwrap();
            *guard
        };

        let mut conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let avg_focus_hours = 4.0;

        let active_commitments: Vec<(i64, String, f64)> = {
            let mut stmt = match conn.prepare(
                "SELECT id, deadline_date, estimated_effort_hours 
                 FROM commitments WHERE status = 'OPEN'"
            ) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let rows = match stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let deadline_str: Option<String> = row.get(1)?;
                let effort: f64 = row.get(2)?;
                Ok((id, deadline_str, effort))
            }) {
                Ok(r) => r,
                Err(_) => continue,
            };
            
            rows.flatten()
                .map(|(id, d_str, eff)| (id, d_str.unwrap_or_default(), eff))
                .filter(|(_, d, _)| !d.is_empty())
                .collect()
        };

        let mut risk_flagged = Vec::new();
        for (id, ds, effort) in active_commitments {
            if let Ok(deadline) = chrono::NaiveDate::parse_from_str(&ds, "%Y-%m-%d") {
                if let Some(dt) = deadline.and_hms_opt(23, 59, 59) {
                    if let Some(deadline_dt) = dt.and_local_timezone(chrono::Local).single() {
                        let diff = deadline_dt.signed_duration_since(st).num_hours() as f64;
                        
                        if diff - effort < avg_focus_hours {
                            let _ = conn.execute("UPDATE commitments SET risk_flagged = 1 WHERE id = ?", [id]);
                            risk_flagged.push(CommitmentPayload {
                                id,
                                time_remaining_hours: diff,
                                estimated_effort_hours: effort,
                            });
                        } else {
                            let _ = conn.execute("UPDATE commitments SET risk_flagged = 0 WHERE id = ?", [id]);
                        }
                    }
                }
            }
        }

        if !risk_flagged.is_empty() {
            let req = SimulatorRequest { commitments: risk_flagged };
            let url = format!("http://127.0.0.1:{}/run_forecast", simulator_port);
            if let Ok(res) = client.post(&url).json(&req).send().await {
                if let Ok(sim_resp) = res.json::<SimulatorResponse>().await {
                    crate::consequence::run_consequence_engine(&mut conn, sim_resp.results, st).await;
                }
            }
        }
    }
}
