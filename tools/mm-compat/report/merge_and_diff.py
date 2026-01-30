#!/usr/bin/env python3
import json
import os

# merge_and_diff.py - Combine discovery results

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    output_dir = os.path.join(script_dir, "../output")
    
    baseline_file = os.path.join(output_dir, "endpoints_baseline.json")
    capture_file = os.path.join(output_dir, "endpoints_capture.json")
    static_file = os.path.join(output_dir, "endpoints_static.json")
    
    final_output = os.path.join(output_dir, "endpoints_final.json")
    diff_output = os.path.join(output_dir, "diff_not_in_openapi.json")
    report_output = os.path.join(output_dir, "priority_report.md")

    # Load data
    baseline = []
    if os.path.exists(baseline_file):
        with open(baseline_file, 'r') as f:
            baseline = json.load(f)
            
    capture = []
    if os.path.exists(capture_file):
        with open(capture_file, 'r') as f:
            capture = json.load(f)
            
    static = []
    if os.path.exists(static_file):
        with open(static_file, 'r') as f:
            static = json.load(f)

    # Union all
    all_endpoints = {}
    
    # Process baseline
    for b in baseline:
        key = f"{b['method']} {b['path']}"
        all_endpoints[key] = {
            "method": b['method'],
            "path": b['path'],
            "sources": ["openapi"],
            "priority": 0
        }
    
    # Process capture
    for c in capture:
        key = f"{c['method']} {c['path']}"
        if key not in all_endpoints:
            all_endpoints[key] = {
                "method": c['method'],
                "path": c['path'],
                "sources": [],
                "priority": 0
            }
        if "capture" not in all_endpoints[key]["sources"]:
            all_endpoints[key]["sources"].append("capture")
        all_endpoints[key]["priority"] += (c['count'] * 10)

    # Process static
    for s in static:
        # Static doesn't usually group by method, assume ANY
        path = s['path']
        # Check if we have any method already for this path
        found = False
        for k in all_endpoints:
            if k.endswith(path):
                if "static" not in all_endpoints[k]["sources"]:
                    all_endpoints[k]["sources"].append("static")
                all_endpoints[k]["priority"] += 5
                found = True
        
        if not found:
            key = f"ANY {path}"
            all_endpoints[key] = {
                "method": "ANY",
                "path": path,
                "sources": ["static"],
                "priority": 2
            }

    # Sort by priority
    sorted_endpoints = sorted(all_endpoints.values(), key=lambda x: x['priority'], reverse=True)
    
    with open(final_output, 'w') as f:
        json.dump(sorted_endpoints, f, indent=2)

    # Diff: Capture but not in OpenAPI
    not_in_openapi = [e for e in sorted_endpoints if "openapi" not in e["sources"] and "capture" in e["sources"]]
    with open(diff_output, 'w') as f:
        json.dump(not_in_openapi, f, indent=2)

    # Generate Priority Report
    with open(report_output, 'w') as f:
        f.write("# MM-Mobile Compatibility Priority Report\n\n")
        f.write("## Top 30 High-Priority Endpoints\n\n")
        f.write("| Method | Path | Sources | Priority Score |\n")
        f.write("|--------|------|---------|----------------|\n")
        for e in sorted_endpoints[:30]:
            f.write(f"| {e['method']} | {e['path']} | {', '.join(e['sources'])} | {e['priority']} |\n")
        
        f.write("\n## Required but missing from OpenAPI\n\n")
        if not_in_openapi:
            for e in not_in_openapi:
                f.write(f"- **{e['method']} {e['path']}** (Found in: {', '.join(e['sources'])})\n")
        else:
            f.write("None found.\n")

    print(f"Generated report at {report_output}")

if __name__ == "__main__":
    main()
