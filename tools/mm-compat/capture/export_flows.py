#!/usr/bin/env python3
import json
import os
import re
from mitmproxy import io
from mitmproxy.exceptions import FlowReadException

# export_flows.py - Process mitmproxy flows into normalized endpoints

def normalize_path(path):
    # Strip query params
    path = path.split('?')[0]
    # Ensure leading /
    if not path.startswith('/'):
        path = '/' + path
    # Collapse duplicate slashes
    path = re.sub(r'/+', '/', path)
    # Parameterize IDs
    path = re.sub(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', ':id', path)
    path = re.sub(r'/[a-z0-9]{26}(?=/|$)', '/:id', path)
    
    return path

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    flow_file = os.path.join(script_dir, "flows.mitm")
    output_json = os.path.join(script_dir, "../output/endpoints_capture.json")
    output_txt = os.path.join(script_dir, "../output/endpoints_capture.txt")

    if not os.path.exists(flow_file):
        print(f"Error: {flow_file} not found.")
        return

    endpoints = {}

    print(f"Reading flows from {flow_file}...")
    with open(flow_file, "rb") as f:
        freader = io.FlowReader(f)
        try:
            for flow in freader.stream():
                if hasattr(flow, "request"):
                    method = flow.request.method
                    path = flow.request.path
                    
                    # Only track /api/v4 and websocket
                    if not (path.startswith('/api/v4') or 'websocket' in path):
                        continue
                        
                    norm_path = normalize_path(path)
                    key = f"{method} {norm_path}"
                    
                    if key not in endpoints:
                        endpoints[key] = {
                            "method": method,
                            "path": norm_path,
                            "count": 0,
                            "samples": []
                        }
                    
                    endpoints[key]["count"] += 1
                    if len(endpoints[key]["samples"]) < 3:
                        endpoints[key]["samples"].append(path)
        except FlowReadException as e:
            print(f"Flow file finished: {e}")

    # Convert to list and sort
    results = sorted(list(endpoints.values()), key=lambda x: x['count'], reverse=True)
    
    os.makedirs(os.path.dirname(output_json), exist_ok=True)
    with open(output_json, 'w') as f:
        json.dump(results, f, indent=2)
        
    with open(output_txt, 'w') as f:
        for r in sorted([f"{r['method']} {r['path']}" for r in results]):
            f.write(r + "\n")

    print(f"Processed {len(results)} unique endpoints to {output_json}")

if __name__ == "__main__":
    main()
