import os
import yaml
import sys

def extract_endpoints(source_dir):
    all_endpoints = []
    
    # Files often refer to $ref which we might not fully resolve, 
    # but we just want path/method/summary.
    
    files = [f for f in os.listdir(source_dir) if f.endswith('.yaml')]
    files.sort()
    
    print(f"Processing {len(files)} files...")
    
    for filename in files:
        filepath = os.path.join(source_dir, filename)
        try:
            with open(filepath, 'r') as f:
                # Some files might have multiple documents or complex structure
                data = yaml.safe_load(f)
                
            if not data:
                continue
                
            # Definitions typically start with the path as key
            for path, ops in data.items():
                if not path.startswith('/'):
                    continue
                    
                for method, details in ops.items():
                    if method.lower() not in ['get', 'post', 'put', 'delete', 'patch', 'options', 'head']:
                        continue
                        
                    summary = details.get('summary', 'No summary')
                    op_id = details.get('operationId', 'N/A')
                    
                    all_endpoints.append({
                        "path": path,
                        "method": method.upper(),
                        "summary": summary,
                        "operationId": op_id,
                        "source": filename
                    })
        except Exception as e:
            print(f"Error processing {filename}: {e}", file=sys.stderr)
            
    return all_endpoints

def generate_markdown(endpoints, output_file):
    with open(output_file, 'w') as f:
        f.write("# Mattermost API v4 Reference\n\n")
        f.write(f"Total endpoints found: {len(endpoints)}\n\n")
        f.write("| Method | Path | Summary | Source |\n")
        f.write("| :--- | :--- | :--- | :--- |\n")
        
        for ep in endpoints:
            f.write(f"| {ep['method']} | `{ep['path']}` | {ep['summary']} | `{ep['source']}` |\n")

if __name__ == "__main__":
    source = "/Users/scolak/Projects/mattermost/api/v4/source"
    output = "mattermost_api_reference.md"
    
    endpoints = extract_endpoints(source)
    generate_markdown(endpoints, output)
    print(f"Done! Extracted {len(endpoints)} endpoints to {output}")
