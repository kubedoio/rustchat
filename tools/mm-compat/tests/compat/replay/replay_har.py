#!/usr/bin/env python3
import json
import argparse
import requests
import time

# replay_har.py - Replay HAR entries against target server

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--har", required=True)
    parser.add_argument("--target", default="http://localhost:3000")
    parser.add_argument("--filter", help="Path filter (e.g. /api/v4)")
    args = parser.parse_args()

    with open(args.har, 'r') as f:
        har_data = json.load(f)

    entries = har_data.get('log', {}).get('entries', [])
    print(f"Loaded {len(entries)} entries from {args.har}")

    results = []
    
    for entry in entries:
        req = entry['request']
        method = req['method']
        url = req['url']
        
        # Parse path from URL
        path = "/" + "/".join(url.split("/")[3:])
        
        if args.filter and args.filter not in path:
            continue

        target_url = args.target.rstrip('/') + path
        print(f"Replaying {method} {path} ... ", end="", flush=True)

        try:
            # Prepare headers
            headers = {h['name']: h['value'] for h in req['headers'] if h['name'].lower() not in ['host', 'content-length']}
            
            # Prepare body
            data = None
            if 'postData' in req:
                data = req['postData'].get('text')

            start_time = time.time()
            resp = requests.request(
                method=method,
                url=target_url,
                headers=headers,
                data=data,
                timeout=10
            )
            latency = (time.time() - start_time) * 1000

            print(f"{resp.status_code} ({latency:.1f}ms)")
            
            results.append({
                "path": path,
                "method": method,
                "expected_status": entry['response']['status'],
                "actual_status": resp.status_code,
                "latency": latency,
                "passed": resp.status_code < 400 or resp.status_code == entry['response']['status']
            })

        except Exception as e:
            print(f"FAILED: {e}")
            results.append({
                "path": path,
                "method": method,
                "error": str(e),
                "passed": False
            })

    # Summary
    passed = len([r for r in results if r.get('passed')])
    total = len(results)
    print(f"\nSummary: {passed}/{total} passed")

    with open("replay_result.json", 'w') as f:
        json.dump(results, f, indent=2)

if __name__ == "__main__":
    main()
