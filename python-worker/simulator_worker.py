import os
import json
import sqlite3
import argparse
from http.server import HTTPServer, BaseHTTPRequestHandler
import math

class SimulatorHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == '/run_forecast':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)
            
            try:
                payload = json.loads(post_data.decode('utf-8'))
                commitments = payload.get("commitments", [])
                
                results = []
                for c in commitments:
                    cid = c.get("id")
                    time_rem = c.get("time_remaining_hours", 0)
                    effort = c.get("estimated_effort_hours", 0)
                    
                    # Deterministic logistic decay simulation (as defined in SRS 4.11.2)
                    k = 0.5
                    diff = time_rem - effort
                    
                    try:
                        p_success = 1.0 / (1.0 + math.exp(-k * diff))
                    except OverflowError:
                        p_success = 0.0 if diff < 0 else 1.0
                        
                    results.append({
                        "id": cid,
                        "p_success": p_success
                    })
                
                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({"results": results}).encode('utf-8'))
                
            except Exception as e:
                print(f"Error in simulation: {e}")
                self.send_response(500)
                self.end_headers()
                self.wfile.write(str(e).encode('utf-8'))
        else:
            self.send_response(404)
            self.end_headers()

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", required=True, type=int)
    args = parser.parse_args()
    
    server_address = ('127.0.0.1', args.port)
    httpd = HTTPServer(server_address, SimulatorHandler)
    print(f"Simulator worker listening on port {args.port}")
    httpd.serve_forever()
