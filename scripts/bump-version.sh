#!/bin/bash

set -e

if [ -z "$1" ]; then
  echo "Usage: ./scripts/bump-version.sh <new_version>"
  exit 1
fi

NEW_VERSION=$1
# Remove 'v' prefix if present
NEW_VERSION=${NEW_VERSION#v}

echo "Bumping version to $NEW_VERSION..."

# Update Backend (Cargo.toml)
if [ -f "backend/Cargo.toml" ]; then
  # Portable in-place edit (works on both GNU and BSD sed).
  sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" backend/Cargo.toml && rm -f backend/Cargo.toml.bak
  echo "Updated backend/Cargo.toml"
fi

# Update Frontend (package.json)
if [ -f "frontend/package.json" ]; then
  # Using a simpler sed approach for package.json to avoid complex JSON parsing dependencies
  sed -i.bak "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" frontend/package.json && rm -f frontend/package.json.bak
  echo "Updated frontend/package.json"
fi

echo "Successfully bumped version to $NEW_VERSION"
echo "Don't forget to commit the changes and tag it with: git tag v$NEW_VERSION"
