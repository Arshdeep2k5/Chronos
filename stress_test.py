import time
import json
import random
import urllib.request
import urllib.error
from concurrent.futures import ThreadPoolExecutor, as_completed

API_URL = "http://127.0.0.1:7899"

def send_event(i):
    payload = {
        "source": "raw",
        "payload": {
            "event_type": "StressTestEvent",
            "source": "StressTester",
            "index": i,
            "timestamp": time.time()
        }
    }
    data = json.dumps(payload).encode('utf-8')
    req = urllib.request.Request(f"{API_URL}/api/perception/ingest", data=data, headers={'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return 1 if resp.status == 200 else 0
    except Exception:
        return 0

def send_query(i):
    endpoints = ["/api/state", "/api/reasoning/forecasts", "/api/health"]
    ep = random.choice(endpoints)
    req = urllib.request.Request(f"{API_URL}{ep}")
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return 1 if resp.status == 200 else 0
    except Exception:
        return 0

def run_stress_test(events=5000, queries=5000):
    print("Starting concurrent stress test...")
    start_time = time.time()
    
    success_events = 0
    fail_events = 0
    success_queries = 0
    fail_queries = 0
    
    with ThreadPoolExecutor(max_workers=50) as executor:
        future_to_type = {}
        for i in range(events):
            future_to_type[executor.submit(send_event, i)] = "event"
        for i in range(queries):
            future_to_type[executor.submit(send_query, i)] = "query"
            
        for future in as_completed(future_to_type):
            req_type = future_to_type[future]
            try:
                res = future.result()
                if req_type == "event":
                    if res: success_events += 1
                    else: fail_events += 1
                else:
                    if res: success_queries += 1
                    else: fail_queries += 1
            except Exception as exc:
                if req_type == "event": fail_events += 1
                else: fail_queries += 1

    duration = time.time() - start_time
    print(f"Test completed in {duration:.2f} seconds.")
    print(f"Telemetry Ingest - Success: {success_events}, Failed: {fail_events}")
    print(f"API Queries      - Success: {success_queries}, Failed: {fail_queries}")

if __name__ == "__main__":
    # We run 10000 of each to heavily stress the locks concurrently
    run_stress_test(events=10000, queries=10000)

