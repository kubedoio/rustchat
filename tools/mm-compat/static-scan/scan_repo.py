#!/usr/bin/env python3
import os
import re
import json
import argparse

# scan_repo.py - Scan mobile repo for API patterns

def normalize_path(path):
    # Ensure leading /
    if not path.startswith('/'):
        path = '/' + path
    # Parameterize IDs
    path = re.sub(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', ':id', path)
    path = re.sub(r'/[a-z0-9]{26}(?=/|$)', '/:id', path)
    return path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", default="mattermost-mobile")
    args = parser.parse_args()

    repo_path = args.repo
    if not os.path.exists(repo_path):
        # Specific check for the user's environment if repo is child of current
        alt_path = os.path.join(os.getcwd(), "mattermost-mobile")
        if os.path.exists(alt_path):
            repo_path = alt_path
        else:
            print(f"Error: Repository path {repo_path} not found.")
            return

    script_dir = os.path.dirname(os.path.abspath(__file__))
    output_json = os.path.join(script_dir, "../output/endpoints_static.json")
    output_txt = os.path.join(script_dir, "../output/endpoints_static.txt")

    # Patterns to look for
    patterns = [
        re.compile(r'/api/v4/[\w/{}]+'),
        re.compile(r'websocket'),
        re.compile(r'/plugins/[\w/]+'),
    ]

    found_endpoints = {}

    print(f"Scanning {repo_path}...")
    for root, _, files in os.walk(repo_path):
        for file in files:
            if file.endswith(('.ts', '.tsx', '.js', '.jsx')):
                file_path = os.path.join(root, file)
                try:
                    with open(file_path, 'r', errors='ignore') as f:
                        for i, line in enumerate(f, 1):
                            for pattern in patterns:
                                matches = pattern.findall(line)
                                for match in matches:
                                    # Normalize common placeholders
                                    norm = match.replace('${', ':').replace('}', '')
                                    norm = normalize_path(norm)
                                    
                                    if norm not in found_endpoints:
                                        found_endpoints[norm] = {
                                            "path": norm,
                                            "references": []
                                        }
                                    
                                    if len(found_endpoints[norm]["references"]) < 5:
                                        found_endpoints[norm]["references"].append({
                                            "file": os.path.relpath(file_path, repo_path),
                                            "line": i
                                        })
                except Exception as e:
                    print(f"Error reading {file_path}: {e}")

    # Sort results
    results = sorted(list(found_endpoints.values()), key=lambda x: x['path'])
    
    os.makedirs(os.path.dirname(output_json), exist_ok=True)
    with open(output_json, 'w') as f:
        json.dump(results, f, indent=2)
        
    with open(output_txt, 'w') as f:
        for r in results:
            f.write(f"ANY {r['path']}\n")

    print(f"Found {len(results)} potential endpoints in {output_json}")

if __name__ == "__main__":
    main()
