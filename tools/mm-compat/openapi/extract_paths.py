#!/usr/bin/env python3
import yaml
import json
import os
import re

# extract_paths.py - Extract and normalize paths from YAML

def normalize_path(path):
    # Remove leading/trailing whitespace
    path = path.strip()
    # Ensure leading /
    if not path.startswith('/'):
        path = '/' + path
    # Collapse duplicate slashes
    path = re.sub(r'/+', '/', path)
    # Parameterize IDs (Mattermost IDs are often 26 chars)
    # Replace UUIDs
    path = re.sub(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', ':id', path)
    # Replace MM 26-char IDs or numeric IDs
    path = re.sub(r'/[a-z0-9]{26}(?=/|$)', '/:id', path)
    
    return path

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    input_file = os.path.join(script_dir, "mattermost-openapi-v4.yaml")
    output_json = os.path.join(script_dir, "../output/endpoints_baseline.json")
    output_txt = os.path.join(script_dir, "../output/endpoints_baseline.txt")

    if not os.path.exists(input_file):
        print(f"Error: {input_file} not found. Run download_openapi.sh first.")
        return

    print(f"Parsing {input_file}...")
    with open(input_file, 'r') as f:
        spec = yaml.safe_load(f)

    endpoints = []
    normal_paths = set()

    paths = spec.get('paths', {})
    for path, methods in paths.items():
        norm_path = normalize_path(path)
        for method in methods:
            if method.lower() in ['get', 'post', 'put', 'delete', 'patch']:
                endpoints.append({
                    "method": method.upper(),
                    "path": norm_path,
                    "original": path
                })
                normal_paths.add(f"{method.upper()} {norm_path}")

    # Sort for determinism
    endpoints.sort(key=lambda x: (x['path'], x['method']))
    
    os.makedirs(os.path.dirname(output_json), exist_ok=True)
    
    with open(output_json, 'w') as f:
        json.dump(endpoints, f, indent=2)
    
    with open(output_txt, 'w') as f:
        for p in sorted(list(normal_paths)):
            f.write(p + "\n")

    print(f"Extracted {len(endpoints)} endpoints to {output_json}")

if __name__ == "__main__":
    main()
