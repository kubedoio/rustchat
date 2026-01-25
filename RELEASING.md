# Release Process
+
+This document describes how to release a new version of RustChat.
+
+## Versioning Strategy
+
+RustChat follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). 
+Both backend and frontend versions are kept in sync.
+
+## How to Release
+
+1. **Bump the version**:
+   Run the helper script to update `Cargo.toml` and `package.json`:
+   ```bash
+   ./scripts/bump-version.sh <new_version>
+   ```
+   Example: `./scripts/bump-version.sh 0.1.0`
+
+2. **Update CHANGELOG.md**:
+   Ensure all notable changes since the last release are documented in `CHANGELOG.md`.
+
+3. **Commit the changes**:
+   ```bash
+   git add .
+   git commit -m "chore: release v0.1.0"
+   ```
+
+4. **Tag the release**:
+   ```bash
+   git tag v0.1.0
+   ```
+
+5. **Push to GitHub**:
+   ```bash
+   git push origin main --tags
+   ```
+
+## CI/CD Automation
+
+Upon pushing a tag starting with `v` (e.g., `v0.1.0`):
+- The **Docker Publish** workflow will build and push the backend and frontend images to GHCR with the version tag.
+- The **Release** workflow will automatically create a GitHub Release with an auto-generated changelog based on pull requests.
+
