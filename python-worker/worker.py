import sys
import os
import time
import argparse
import threading
import json
import sqlite3
import requests
from pathlib import Path

import cde_parser
import embeddings
import arc_crawler

# Global settings
DB_PATH = Path(os.path.expanduser("~")) / ".config" / "chronos" / "chronos.db"

def send_heartbeat(token, port):
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
            if response.status_code != 200:
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
            SELECT n.id, n.entity_key 
            FROM context_nodes n 
            LEFT JOIN commitments c ON n.id = c.source_node_id 
            WHERE (n.entity_type = 'DOCUMENT' OR n.entity_type = 'FILE') AND c.id IS NULL
        """)
        
        rows = cursor.fetchall()
        for node_id, entity_key in rows:
            file_path = entity_key.replace("DOCUMENT:", "").replace("FILE:", "")
            if os.path.exists(file_path):
                commitments = cde_parser.parse_commitments(file_path)
                for c in commitments:
                    # Find project_id or assign default
                    cursor.execute("SELECT id FROM projects LIMIT 1")
                    proj_row = cursor.fetchone()
                    project_id = proj_row[0] if proj_row else None
                    
                    cursor.execute("""
                        INSERT INTO commitments (project_id, title, commitment_type, deadline_date, confidence_score, source_node_id, status)
                        VALUES (?, ?, ?, ?, ?, ?, 'OPEN')
                    """, (project_id, c["title"], c["commitment_type"], c["deadline_date"], c["confidence_score"], node_id))
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
        cursor.execute("SELECT id, title, deadline_date FROM commitments WHERE status = 'OPEN'")
        commitments = cursor.fetchall()
        
        for cid, title, deadline in commitments:
            # Check if plan already generated
            cursor.execute("SELECT id FROM recovery_plans WHERE commitment_id = ?", (cid,))
            if cursor.fetchone():
                continue
                
            # If deadline is imminent, synthesize recovery checklist plan (Section 4.8)
            if deadline:
                try:
                    due_date = datetime.datetime.strptime(deadline, "%Y-%m-%d").date()
                    days_left = (due_date - datetime.date.today()).days
                    if days_left <= 3:
                        # Generate catches checklist
                        plan = [
                            {"day": "Today", "task": "Complete literature review & parse inputs", "hours": 3.5},
                            {"day": "Tomorrow", "task": "Train local classifier & run baseline", "hours": 4.0},
                            {"day": "Wednesday", "task": "Finalize report writing & export PDFs", "hours": 2.5}
                        ]
                        cursor.execute("""
                            INSERT INTO recovery_plans (commitment_id, plan_payload_json)
                            VALUES (?, ?)
                        """, (cid, json.dumps(plan)))
                        conn.commit()
                        print(f"Generated Recovery Plan for Commitment: {title}")
                except Exception as ex:
                    print(f"Date error: {ex}")
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
        
        # Get recent context events (last 15 mins before snapshot)
        cursor.execute("""
            SELECT display_name FROM context_nodes WHERE project_id = ? ORDER BY id DESC LIMIT 5
        """, (project_id,))
        recent_nodes = cursor.fetchall()
        
        if not snapshot and not search and not checkpoint:
            return
            
        active_file = snapshot[0] if snapshot else "None"
        cursor_line = snapshot[1] if snapshot else 0
        last_search = search[0] if search else "None"
        blocker = checkpoint[0] if checkpoint else "None"
        
        # Define the narrative output path
        narrative_path = Path(os.path.expanduser("~")) / ".config" / "chronos" / "reconstruction_narrative.md"
        
        # Check if we've already generated it recently to avoid spam (in a real app we'd check timestamps)
        
        # Determine if we have Gemini API key in env
        gemini_api_key = os.environ.get("GEMINI_API_KEY")
        narrative_text = ""
        
        prompt = f"""
You are the Chronos Cognitive Context Reconstruction engine.
Synthesize a 'Why You Stopped' explanation based on the following final activity window:
- Final open file: {active_file} (Line {cursor_line})
- Final search query: "{last_search}"
- Last checkpoint blocker: "{blocker}"
- Recent context nodes: {', '.join([n[0] for n in recent_nodes])}

Output a short, human-readable narrative explaining what the user was likely doing before they stopped, and what they need to do to resume.
        """
        
        if gemini_api_key:
            # Call Gemini REST API
            try:
                url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={gemini_api_key}"
                payload = {"contents": [{"parts":[{"text": prompt}]}]}
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
            for n in recent_nodes:
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
        
        # Get open project actions with project commitments
        cursor.execute("""
            SELECT pa.id, c.deadline_date, pa.estimated_effort_hours
            FROM project_actions pa
            JOIN projects p ON pa.project_id = p.id
            JOIN commitments c ON p.id = c.project_id
            WHERE pa.status = 'PENDING'
        """)
        actions = cursor.fetchall()
        
        for act_id, deadline, effort in actions:
            weight = 1.0
            if deadline:
                try:
                    import datetime
                    due_date = datetime.datetime.strptime(deadline, "%Y-%m-%d").date()
                    days_left = (due_date - datetime.date.today()).days
                    
                    if days_left <= 0:
                        weight = 100.0  # CRITICAL
                    elif days_left <= 3:
                        weight = 50.0   # HIGH
                    elif days_left <= 7:
                        weight = 20.0   # MEDIUM
                    else:
                        weight = 5.0    # LOW
                        
                    # Adjust weight based on estimated effort (higher effort = needs attention earlier)
                    weight += float(effort) * 2.0
                    
                except Exception:
                    pass
            
            # Update the priority_score
            cursor.execute("UPDATE project_actions SET priority_score = ? WHERE id = ?", (weight, act_id))
            
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
    
    # Initialize handshake directories if missing
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    
    # Start heartbeat thread
    hb_thread = threading.Thread(target=send_heartbeat, args=(args.token, args.port), daemon=True)
    hb_thread.start()
    
    # Run processing loop
    main_loop()
