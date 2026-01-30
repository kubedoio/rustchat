#!/bin/bash
# start_proxy.sh - Start mitmproxy to capture MM Mobile traffic

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
OUTPUT_DIR="$DIR/../output"
mkdir -p "$OUTPUT_DIR"

FLOW_FILE="$DIR/flows.mitm"

echo "Starting mitmproxy on port 8080..."
echo "Traffic will be saved to $FLOW_FILE"

# Start mitmdump in the background or use mitmproxy for interactive
# Here we use mitmproxy for better visibility if running manually
mitmproxy --listen-port 8080 --save-stream-file "$FLOW_FILE" "$@"

echo "Proxy stopped. Processing flows..."
python3 "$DIR/export_flows.py"
