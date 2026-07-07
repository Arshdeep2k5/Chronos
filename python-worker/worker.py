import sys
import re
import os
import time
import argparse
import threading
import json
import sqlite3
import requests
import datetime
from pathlib import Path

import cde_parser
import embeddings
import arc_crawler

def redact_pii(text):
    # Redact email addresses
    text = re.sub(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+', '[EMAIL_REDACTED]', text)
    # Redact IP addresses
    text = re.sub(r'\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b', '[IP_REDACTED]', text)
    # Redact credentials/tokens (e.g. bearer tokens, API keys, webhook secrets)
    text = re.sub(r'(?i)(api[_-]?key|secret|token|password|auth|auth_token)\s*[:=]\s*["\']?[a-zA-Z0-9_\-]{16,}["\']?', r'\1: [CREDENTIAL_REDACTED]', text)
    return text

# Global settings
DB_PATH = Path(os.path.expanduser("~")) / ".config" / "chronos" / "chronos.db"

PORT = None
TOKEN = None
SENTINEL_PATH = DB_PATH.parent / "cde_active_parse.json"

def calculate_file_hash(file_path):
    import hashlib
    try:
        hasher = hashlib.sha256()
        with open(file_path, 'rb') as f:
            while chunk := f.read(8192):
                hasher.update(chunk)
        return hasher.hexdigest()
    except Exception:
        return hashlib.sha256(str(file_path).encode('utf-8')).hexdigest()

def parse_iso_datetime(date_str):
    if not date_str:
        return None
    date_str = date_str.strip()
    if date_str.endswith('Z'):
        date_str = date_str[:-1] + '+00:00'
    for fmt in ("%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M:%S.%f%z", "%Y-%m-%d %H:%M:%S%z", "%Y-%m-%d %H:%M:%S.%f%z", "%Y-%m-%d"):
        try:
            dt = datetime.datetime.strptime(date_str, fmt)
            if "%z" not in fmt:
                dt = dt.replace(tzinfo=datetime.timezone.utc)
            return dt
        except ValueError:
            continue
    # Try naive parses and assume UTC
    for fmt in ("%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M:%S.%f", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M:%S.%f"):
        try:
            dt = datetime.datetime.strptime(date_str, fmt)
            return dt.replace(tzinfo=datetime.timezone.utc)
        except ValueError:
            continue
    return None

def to_utc(dt):
    if dt is None:
        return None
    if dt.tzinfo is not None:
        return dt.astimezone(datetime.timezone.utc).replace(tzinfo=None)
    return dt

def get_simulated_time():
    if not PORT or not TOKEN:
        return datetime.datetime.now(datetime.timezone.utc)
    url = f"http://localhost:{PORT}/api/system-status"
    headers = {
        "Authorization": f"Bearer {TOKEN}",
        "Content-Type": "application/json"
    }
    try:
        resp = requests.get(url, headers=headers, timeout=2)
        if resp.status_code == 200:
            data = resp.json()
            st_str = data.get("systemTime")
            if st_str:
                dt = parse_iso_datetime(st_str)
                if dt:
                    return dt
    except Exception as e:
        print(f"Error fetching simulated time: {e}")
    return datetime.datetime.now(datetime.timezone.utc)

def check_and_recover_dlq():
    if SENTINEL_PATH.exists():
        print(f"CDE Sentinel file detected at {SENTINEL_PATH}. Worker must have crashed last run.")
        try:
            with open(SENTINEL_PATH, 'r', encoding='utf-8') as f:
                data = json.load(f)
            
            source_uri = data.get("source_uri")
            payload_hash = data.get("payload_hash")
            worker_type = data.get("worker_type", "python_nlp")
            failure_reason = data.get("failure_reason", "CRASH_DURING_EXTRACTION")
            
            if source_uri and payload_hash:
                print(f"Inserting crashed file to DLQ: {source_uri} (hash: {payload_hash})")
                conn = sqlite3.connect(DB_PATH)
                cursor = conn.cursor()
                cursor.execute("""
                    INSERT OR IGNORE INTO dead_letter_queue (source_uri, payload_hash, worker_type, failure_reason)
                    VALUES (?, ?, ?, ?)
                """, (source_uri, payload_hash, worker_type, failure_reason))
                conn.commit()
                conn.close()
        except Exception as e:
            print(f"Failed to recover sentinel: {e}")
        finally:
            try:
                SENTINEL_PATH.unlink()
            except Exception:
                pass

PROCESSING_PAUSED = False
IS_IDLE = True

def send_heartbeat(token, port):
    global PROCESSING_PAUSED, IS_IDLE
    url = f"http://localhost:{port}/heartbeat"
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }
    payload = {
        "status": "ALIVE",
        "worker": "python_nlp",
        "memory_mb": 114
    }
    
    while True:
        try:
            response = requests.post(url, headers=headers, json=payload, timeout=3)
            if response.status_code == 200:
                try:
                    data = response.json()
                    PROCESSING_PAUSED = data.get("pause", False)
                    IS_IDLE = data.get("is_idle", True)
                    if PROCESSING_PAUSED:
                        cpu = data.get("cpu_usage", 0.0)
                        ram = data.get("ram_usage", 0.0)
                        bat = data.get("battery_percentage")
                        bat_str = f"{bat}%" if bat is not None else "N/A"
                        print(f"[Compute Gatekeeper] System resources overloaded (CPU: {cpu:.1f}%, RAM: {ram:.1f}%, Battery: {bat_str}). Background jobs paused.")
                except Exception as ex:
                    pass
            else:
                print(f"Heartbeat rejected: {response.status_code}")
        except Exception as e:
            print(f"Heartbeat connection error: {e}")
        time.sleep(5)

def process_unembedded_nodes():
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Find nodes without embeddings
        cursor.execute("""
            SELECT n.id, n.display_name, n.entity_key, n.entity_type 
            FROM context_nodes n 
            LEFT JOIN context_embeddings e ON n.id = e.node_id 
            WHERE e.node_id IS NULL
        """)
        
        rows = cursor.fetchall()
        for node_id, display_name, entity_key, entity_type in rows:
            text_to_embed = display_name
            
            # If it's a file, we can read a snippet
            if entity_type == 'FILE' or entity_type == 'DOCUMENT':
                file_path = entity_key.replace("FILE:", "").replace("DOCUMENT:", "")
                if os.path.exists(file_path):
                    text_to_embed = cde_parser.extract_text_from_file(file_path)[:1000] or display_name
            
            emb = embeddings.get_embedding(text_to_embed)
            emb_str = json.dumps(emb)
            
            # Try inserting into context_embeddings. We handle both virtual table vec0 and standard fallback table.
            try:
                cursor.execute(
                    "INSERT INTO context_embeddings (node_id, embedding) VALUES (?, ?)", 
                    (node_id, emb_str)
                )
            except sqlite3.OperationalError:
                # If virtual table is vec0, it expects float array
                # For sqlite-vec virtual table vec0, we might need a serialized float array or specific insertion
                # In standard SQLite fallback, text is fine. Let's do a fallback insert
                cursor.execute(
                    "INSERT OR REPLACE INTO context_embeddings (node_id, embedding) VALUES (?, ?)",
                    (node_id, emb_str)
                )
            conn.commit()
            print(f"Computed embedding for node {node_id}: {display_name}")
            
    except Exception as e:
        print(f"Error in process_unembedded_nodes: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def process_cde_extractions():
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Find document/file nodes that haven't been parsed for commitments
        cursor.execute("""
            SELECT n.id, n.entity_key, n.project_id 
            FROM context_nodes n 
            LEFT JOIN commitments c ON n.id = c.source_node_id 
            WHERE (n.entity_type = 'DOCUMENT' OR n.entity_type = 'FILE') AND c.id IS NULL
        """)
        
        rows = cursor.fetchall()
        for node_id, entity_key, project_id in rows:
            file_path = entity_key.replace("DOCUMENT:", "").replace("FILE:", "")
            if not os.path.exists(file_path):
                continue
                
            file_hash = calculate_file_hash(file_path)
            
            # Check if present in DLQ
            cursor.execute("SELECT id FROM dead_letter_queue WHERE source_uri = ? OR payload_hash = ?", (file_path, file_hash))
            if cursor.fetchone():
                print(f"Skipping CDE extraction for {file_path} (present in DLQ)")
                continue
                
            # Write sentinel
            try:
                with open(SENTINEL_PATH, 'w', encoding='utf-8') as f:
                    json.dump({
                        "source_uri": file_path,
                        "payload_hash": file_hash,
                        "worker_type": "python_nlp",
                        "failure_reason": "CRASH_DURING_EXTRACTION"
                    }, f)
            except Exception as e:
                print(f"Warning: could not write sentinel file: {e}")
                
            try:
                commitments = cde_parser.parse_commitments(file_path)
                # Remove sentinel since parsing completed successfully
                if SENTINEL_PATH.exists():
                    SENTINEL_PATH.unlink()
            except Exception as e:
                print(f"CDE Extraction crashed for {file_path}: {e}")
                cursor.execute("""
                    INSERT OR IGNORE INTO dead_letter_queue (source_uri, payload_hash, worker_type, failure_reason)
                    VALUES (?, ?, ?, ?)
                """, (file_path, file_hash, "python_nlp", f"CRASH_DURING_EXTRACTION: {str(e)}"))
                conn.commit()
                if SENTINEL_PATH.exists():
                    SENTINEL_PATH.unlink()
                continue
                
            for c in commitments:
                if c.get("confidence_score", 0.0) < 0.60:
                    continue
                    
                target_proj_id = project_id
                if target_proj_id is None:
                    cursor.execute("SELECT id FROM projects LIMIT 1")
                    proj_row = cursor.fetchone()
                    target_proj_id = proj_row[0] if proj_row else None
                    
                cursor.execute("""
                    INSERT INTO commitments (project_id, title, commitment_type, deadline_date, confidence_score, source_node_id, status, estimated_effort_hours)
                    VALUES (?, ?, ?, ?, ?, ?, 'OPEN', ?)
                """, (target_proj_id, c["title"], c["commitment_type"], c["deadline_date"], c["confidence_score"], node_id, c.get("estimated_effort_hours", 2.5)))
                conn.commit()
                print(f"CDE Discovered Commitment: {c['title']} | Deadline: {c['deadline_date']}")
    except Exception as e:
        print(f"Error in process_cde_extractions: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def reconstruct_research_sessions():
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Get unassigned browser sessions
        cursor.execute("""
            SELECT id, url, page_title, domain, visit_started_at 
            FROM browser_sessions 
            WHERE project_id IS NULL
        """)
        sessions = cursor.fetchall()
        if not sessions:
            return
            
        # Standard group logic: group sessions within 15 minutes
        # We find a target project or assign a default project
        cursor.execute("SELECT id FROM projects LIMIT 1")
        proj_row = cursor.fetchone()
        if not proj_row:
            return
        project_id = proj_row[0]
        
        for sess in sessions:
            sess_id = sess[0]
            cursor.execute("UPDATE browser_sessions SET project_id = ? WHERE id = ?", (project_id, sess_id))
            conn.commit()
            
            # Create a context node for research session
            entity_key = f"SESSION:{sess_id}"
            cursor.execute("""
                INSERT OR IGNORE INTO context_nodes (project_id, entity_key, entity_type, display_name)
                VALUES (?, ?, 'RESEARCH_SESSION', ?)
            """, (project_id, entity_key, f"Research Session: {sess[2][:30]}"))
            conn.commit()
            
    except Exception as e:
        print(f"Error in reconstruct_research_sessions: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def run_recovery_planner():
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Check for open commitments
        cursor.execute("SELECT id, title, deadline_date, project_id FROM commitments WHERE status = 'OPEN'")
        commitments = cursor.fetchall()
        
        for cid, title, deadline, project_id in commitments:
            # Check if plan already generated
            cursor.execute("SELECT id FROM recovery_plans WHERE commitment_id = ?", (cid,))
            if cursor.fetchone():
                continue
                
            # If deadline is imminent, synthesize recovery checklist plan
            if deadline:
                try:
                    due_date = datetime.datetime.strptime(deadline, "%Y-%m-%d").date()
                    days_left = max(1, (due_date - datetime.date.today()).days)
                    if days_left <= 7:
                        # Fetch actual project actions instead of mock
                        cursor.execute("SELECT action_text, estimated_effort_hours FROM project_actions WHERE status = 'PENDING' AND project_id = ? ORDER BY priority_score DESC", (project_id,))
                        actions = cursor.fetchall()
                        
                        plan = []
                        if not actions:
                            plan.append({"day": "Today", "task": "Review project requirements and begin work.", "hours": 2.0})
                        else:
                            day_offsets = ["Today", "Tomorrow", "Day 3", "Day 4", "Day 5", "Day 6", "Day 7"]
                            current_day_idx = 0
                            hours_today = 0
                            
                            for act_text, effort in actions:
                                act_effort = effort if effort else 1.0
                                # distribute actions across days, capping at ~4 hours per day
                                plan.append({
                                    "day": day_offsets[current_day_idx] if current_day_idx < len(day_offsets) else "Later",
                                    "task": act_text,
                                    "hours": round(act_effort, 1)
                                })
                                hours_today += act_effort
                                if hours_today >= 4.0 and current_day_idx < days_left - 1:
                                    current_day_idx += 1
                                    hours_today = 0
                                    
                        cursor.execute("""
                            INSERT INTO recovery_plans (commitment_id, plan_payload_json)
                            VALUES (?, ?)
                        """, (cid, json.dumps(plan)))
                        conn.commit()
                        print(f"Generated Dynamic Recovery Plan for Commitment: {title}")
                except Exception as ex:
                    print(f"Date error in recovery planner: {ex}")
    except Exception as e:
        print(f"Error in run_recovery_planner: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def run_background_arc():
    # If system is idle or triggered, we can crawl arXiv/GitHub
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Look up latest search query
        cursor.execute("SELECT query_text FROM search_queries ORDER BY id DESC LIMIT 1")
        row = cursor.fetchone()
        if not row:
            return
            
        query_text = row[0]
        
        # Check if research briefs already generated for this project
        cursor.execute("SELECT id FROM projects LIMIT 1")
        proj_row = cursor.fetchone()
        if not proj_row:
            return
        project_id = proj_row[0]
        
        cursor.execute("SELECT id FROM autonomous_research_briefs WHERE project_id = ?", (project_id,))
        if cursor.fetchone():
            return
            
        print(f"ARC Active: Crawling arXiv and GitHub for query: '{query_text}'")
        brief_data = arc_crawler.generate_briefing(query_text)
        if brief_data:
            cursor.execute("""
                INSERT INTO autonomous_research_briefs (project_id, brief_payload_json)
                VALUES (?, ?)
            """, (project_id, json.dumps(brief_data)))
            conn.commit()
            print("Successfully populated autonomous_research_briefs.")
            
    except Exception as e:
        print(f"Error in run_background_arc: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def run_narrative_reconstruction():
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Check for active project
        cursor.execute("SELECT id FROM projects LIMIT 1")
        proj_row = cursor.fetchone()
        if not proj_row:
            return
        project_id = proj_row[0]

        # Get last workspace snapshot
        cursor.execute("""
            SELECT active_file_path, cursor_line, cursor_column, captured_at 
            FROM workspace_snapshots 
            WHERE project_id = ? 
            ORDER BY id DESC LIMIT 1
        """, (project_id,))
        snapshot = cursor.fetchone()

        # Get last search query
        cursor.execute("""
            SELECT sq.query_text 
            FROM search_queries sq
            JOIN browser_sessions bs ON sq.browser_session_id = bs.id
            WHERE bs.project_id = ?
            ORDER BY sq.id DESC LIMIT 1
        """, (project_id,))
        search = cursor.fetchone()

        # Get last checkpoint blocker
        cursor.execute("""
            SELECT blocked_text 
            FROM project_checkpoints 
            WHERE project_id = ? 
            ORDER BY id DESC LIMIT 1
        """, (project_id,))
        checkpoint = cursor.fetchone()
        
        # PCM Tier 1: Context events from the last 48 hours
        cursor.execute("""
            SELECT cn.display_name, ce.captured_at
            FROM context_events ce
            JOIN context_nodes cn ON ce.node_id = cn.id
            WHERE cn.project_id = ? AND ce.captured_at >= datetime('now', '-48 hours')
            ORDER BY ce.id DESC LIMIT 20
        """, (project_id,))
        nodes_48h = cursor.fetchall()
        
        # PCM Tier 2: Last 3 checkpoints
        cursor.execute("""
            SELECT accomplished_text, blocked_text, next_steps_text, created_at
            FROM project_checkpoints
            WHERE project_id = ?
            ORDER BY id DESC LIMIT 3
        """, (project_id,))
        checkpoints_3 = cursor.fetchall()
        
        # PCM Tier 3: Top 5 semantic matches
        active_file = snapshot[0] if snapshot else "None"
        cursor_line = snapshot[1] if snapshot else 0
        last_search = search[0] if search else "None"
        blocker = checkpoint[0] if checkpoint else "None"
        
        semantic_matches = []
        target_text = active_file if active_file != "None" else last_search
        if target_text and target_text != "None":
            try:
                target_emb = embeddings.get_embedding(target_text)
                cursor.execute("""
                    SELECT cn.display_name, ce.embedding
                    FROM context_nodes cn
                    JOIN context_embeddings ce ON cn.id = ce.node_id
                    WHERE cn.project_id = ? AND cn.display_name != ?
                """, (project_id, target_text))
                emb_rows = cursor.fetchall()
                
                import numpy as np
                scored = []
                for name, emb_str in emb_rows:
                    try:
                        emb = json.loads(emb_str)
                        score = np.dot(target_emb, emb)
                        scored.append((name, score))
                    except:
                        pass
                scored.sort(key=lambda x: x[1], reverse=True)
                semantic_matches = [name for name, score in scored[:5]]
            except Exception as e:
                print(f"Error calculating semantic matches: {e}")
                
        if not snapshot and not search and not checkpoint:
            return
            
        # Define the narrative output path
        narrative_path = Path(os.path.expanduser("~")) / ".config" / "chronos" / "reconstruction_narrative.md"
        
        # Determine if we have Gemini API key in env
        gemini_api_key = os.environ.get("GEMINI_API_KEY")
        narrative_text = ""
        
        # Format strings for the prompt
        nodes_48h_str = "\n".join([f"- {n[0]} ({n[1]})" for n in nodes_48h]) if nodes_48h else "None"
        semantic_str = "\n".join([f"- {m}" for m in semantic_matches]) if semantic_matches else "None"
        
        checkpoint_blocks = []
        for i, cp in enumerate(checkpoints_3):
            checkpoint_blocks.append(
                f"Checkpoint {i+1} ({cp[3]}):\n"
                f"  - Accomplished: {cp[0]}\n"
                f"  - Blocker: {cp[1]}\n"
                f"  - Next Steps: {cp[2]}"
            )
        checkpoints_str = "\n\n".join(checkpoint_blocks) if checkpoint_blocks else "None"
        
        prompt = f"""
You are the Chronos Cognitive Context Reconstruction engine.
Synthesize a 'Why You Stopped' explanation based on the following Tiered RAG (PCM) context:

[1. FINAL ACTIVE WINDOW]
- Final open file: {active_file} (Line {cursor_line})
- Final search query: "{last_search}"
- Last checkpoint blocker: "{blocker}"

[2. CONTEXT HISTORY (LAST 48 HOURS)]
{nodes_48h_str}

[3. RECENT CHECKPOINTS (LAST 3)]
{checkpoints_str}

[4. SEMANTICALLY RELATED CONTEXT NODES]
{semantic_str}

Output a short, human-readable narrative explaining what the user was doing before they stopped, what blocker they faced, and what concrete steps they need to resume.
"""
        # Apply Asymmetrical Redaction right at the API boundary
        redacted_prompt = redact_pii(prompt)
        
        if gemini_api_key:
            # Call Gemini REST API
            try:
                url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={gemini_api_key}"
                payload = {"contents": [{"parts":[{"text": redacted_prompt}]}]}
                resp = requests.post(url, json=payload, headers={"Content-Type": "application/json"})
                if resp.status_code == 200:
                    data = resp.json()
                    narrative_text = data.get("candidates", [{}])[0].get("content", {}).get("parts", [{}])[0].get("text", "")
            except Exception as e:
                print(f"Gemini API error: {e}")
        
        # Fallback / Output generation
        if not narrative_text:
            narrative_text = f"**Likely Stopping Point:** You were likely blocked on: {blocker}\n\n"
            narrative_text += "**Final Activity Window:**\n"
            narrative_text += f"- Final open file: `{active_file}` (Line {cursor_line})\n"
            narrative_text += f"- Final search query: \"{last_search}\"\n"
            narrative_text += f"- Last checkpoint blocker: \"{blocker}\"\n"
            narrative_text += "\n**Context Nodes:**\n"
            for n in nodes_48h:
                narrative_text += f"- {n[0]}\n"
                
        with open(narrative_path, 'w', encoding='utf-8') as f:
            f.write(narrative_text)
            
    except Exception as e:
        print(f"Error in run_narrative_reconstruction: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def calculate_attention_weights():
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        sim_time = get_simulated_time()
        sim_time_utc = to_utc(sim_time)
        
        # We query all pending actions
        cursor.execute("SELECT id, project_id, action_text, estimated_effort_hours FROM project_actions WHERE status = 'PENDING'")
        actions = cursor.fetchall()
        
        for act_id, pid, action_text, effort in actions:
            # 1. p_prox
            p_prox = 0.0
            cursor.execute("""
                SELECT target_date FROM project_deadlines 
                WHERE project_id = ? ORDER BY target_date ASC LIMIT 1
            """, (pid,))
            dl_row = cursor.fetchone()
            if dl_row and dl_row[0]:
                dl_dt = parse_iso_datetime(dl_row[0])
                if dl_dt:
                    dl_utc = to_utc(dl_dt)
                    td = dl_utc - sim_time_utc
                    hours_remaining = td.total_seconds() / 3600.0
                    hours_remaining = max(0.0, hours_remaining)
                    p_prox = max(0.0, 1.0 - (hours_remaining / 168.0))
            
            # 2. avg_attn_norm
            avg_attn_norm = 0.0
            # Get all context nodes for this project
            cursor.execute("SELECT id FROM context_nodes WHERE project_id = ?", (pid,))
            nodes = cursor.fetchall()
            if nodes:
                total_attn = 0.0
                count = 0
                forty_eight_hours_ago = sim_time_utc - datetime.timedelta(hours=48)
                forty_eight_hours_ago_str = forty_eight_hours_ago.isoformat() + "Z"
                
                for (nid,) in nodes:
                    cursor.execute("""
                        SELECT SUM(interaction_duration) FROM context_events 
                        WHERE node_id = ? AND event_type IN ('TAB_FOCUS', 'OPENED', 'EDITED') AND captured_at >= ?
                    """, (nid, forty_eight_hours_ago_str))
                    focus_sec = cursor.fetchone()[0] or 0
                    
                    cursor.execute("""
                        SELECT COUNT(*) FROM context_events 
                        WHERE node_id = ? AND event_type = 'EDITED' AND captured_at >= ?
                    """, (nid, forty_eight_hours_ago_str))
                    edits = cursor.fetchone()[0] or 0
                    
                    cursor.execute("""
                        SELECT COUNT(*) FROM context_events 
                        WHERE node_id = ? AND captured_at >= ?
                    """, (nid, forty_eight_hours_ago_str))
                    revisits = cursor.fetchone()[0] or 0
                    
                    import math
                    w_attn_node = 0.50 * math.log1p(focus_sec) + 0.35 * edits + 0.15 * revisits
                    total_attn += w_attn_node
                    count += 1
                
                if count > 0:
                    avg_attn_norm = min(1.0, total_attn / count / 5.0)
            
            # 3. c_urg
            c_urg = 0.0
            cursor.execute("""
                SELECT blocked_text, next_steps_text FROM project_checkpoints 
                WHERE project_id = ? ORDER BY id DESC LIMIT 1
            """, (pid,))
            cp_row = cursor.fetchone()
            if cp_row:
                blocked = (cp_row[0] or "").lower()
                next_steps = (cp_row[1] or "").lower()
                action_lower = action_text.lower()
                if action_lower in blocked or action_lower in next_steps:
                    c_urg = 1.0
            
            # 4. e_achieve
            e_achieve = 1.0 - min(1.0, effort / 168.0)
            
            # AP calculation
            w_prox_val = 0.40
            w_attn_val = 0.20
            w_urg_val = 0.25
            w_effort_val = 0.15
            
            raw_ap = (w_prox_val * p_prox) + (w_attn_val * avg_attn_norm) + (w_urg_val * c_urg) + (w_effort_val * e_achieve)
            score = round(round(raw_ap * 10.0, 2), 1)
            score = max(0.0, min(10.0, score))
            
            # Update in DB
            cursor.execute("UPDATE project_actions SET priority_score = ? WHERE id = ?", (score, act_id))
            
        conn.commit()
    except Exception as e:
        print(f"Error in calculate_attention_weights: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def run_project_matching():
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Get unassigned nodes
        cursor.execute("""
            SELECT id, display_name FROM context_nodes WHERE project_id IS NULL
        """)
        unassigned_nodes = cursor.fetchall()
        if not unassigned_nodes:
            return
            
        # Get projects
        cursor.execute("SELECT id, project_name FROM projects")
        projects = cursor.fetchall()
        if not projects:
            return
            
        import numpy as np
        
        proj_embs = {}
        for p_id, p_name in projects:
            proj_embs[p_id] = np.array(embeddings.get_embedding(p_name))
            
        for node_id, display_name in unassigned_nodes:
            node_emb = np.array(embeddings.get_embedding(display_name))
            
            best_proj = None
            best_score = -1.0
            
            for p_id, p_emb in proj_embs.items():
                score = np.dot(node_emb, p_emb)
                if score > best_score:
                    best_score = score
                    best_proj = p_id
                    
            if best_proj is not None and best_score > 0.4:
                cursor.execute("UPDATE context_nodes SET project_id = ? WHERE id = ?", (best_proj, node_id))
                
        conn.commit()
    except Exception as e:
        print(f"Error in run_project_matching: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

def main_loop():
    while True:
        if PROCESSING_PAUSED:
            time.sleep(3)
            continue
            
        if not IS_IDLE:
            # If user is active, only run critical lightweight checks (like new file parsing)
            process_cde_extractions()
            time.sleep(5)
            continue
            
        # When system is idle, run heavy analytics tasks
        process_unembedded_nodes()
        process_cde_extractions()
        reconstruct_research_sessions()
        run_recovery_planner()
        run_background_arc()
        run_narrative_reconstruction()
        calculate_attention_weights()
        run_project_matching()
        time.sleep(3)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--token", required=True)
    parser.add_argument("--port", required=True, type=int)
    args = parser.parse_args()
    
    print(f"Python worker active. Connecting to localhost:{args.port} using token.")
    
    PORT = args.port
    TOKEN = args.token
    
    # Initialize handshake directories if missing
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    
    # Run DLQ recovery before starting loop
    check_and_recover_dlq()
    
    # Start heartbeat thread
    hb_thread = threading.Thread(target=send_heartbeat, args=(args.token, args.port), daemon=True)
    hb_thread.start()
    
    # Run processing loop
    main_loop()
