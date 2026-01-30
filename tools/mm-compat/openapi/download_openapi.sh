#!/bin/bash
# download_openapi.sh - Download the canonical MM OpenAPI spec

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SPEC_URL="https://developers.mattermost.com/mattermost-openapi-v4.yaml"
OUTPUT="$DIR/mattermost-openapi-v4.yaml"

echo "Downloading Mattermost v4 OpenAPI spec..."
curl -sSL "$SPEC_URL" -o "$OUTPUT"

if [ $? -eq 0 ]; then
  echo "Successfully downloaded to $OUTPUT"
else
  echo "Failed to download spec."
  exit 1
fi
