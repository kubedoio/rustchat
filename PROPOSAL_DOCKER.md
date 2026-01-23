# Docker Image Versioning & Multi-Arch Proposal

## Overview

This proposal outlines the strategy for building, versioning, and publishing Docker images for the `rustchat` project. The goal is to provide reliable, versioned images for both x86_64 (amd64) and ARM64 architectures, hosted on GitHub Container Registry (GHCR).

## 1. Versioning Strategy

We will adopt a semantic versioning strategy coupled with Git metadata for traceability.

### Image Tags

- **`latest`**: Always points to the most recent commit on the `main` branch.
- **`main`**: Also points to the most recent commit on the `main` branch (alias for latest on main).
- **`vX.Y.Z`**: Created when a Git tag `vX.Y.Z` is pushed. This is for stable releases.
- **`sha-<short_sha>`**: Created for every commit on `main` to allow pinning to specific commits.
- **`pr-<number>`**: (Optional) Can be built for Pull Requests for testing purposes, but usually ephemeral.

### Metadata (OCI Labels)

We will embed OCI-compliant labels into the Docker images for better introspection:

- `org.opencontainers.image.title`: "rustchat-backend" / "rustchat-frontend"
- `org.opencontainers.image.description`: Project description
- `org.opencontainers.image.source`: Repository URL
- `org.opencontainers.image.version`: The version tag or SHA
- `org.opencontainers.image.created`: Build timestamp
- `org.opencontainers.image.revision`: Git commit SHA
- `org.opencontainers.image.licenses`: "MIT"

## 2. Multi-Architecture Support

To support a wide range of devices (including Apple Silicon M1/M2/M3 and Raspberry Pi 4/5, as well as standard servers), we will build for:

- **`linux/amd64`** (x86_64)
- **`linux/arm64`** (aarch64)

This will be achieved using Docker Buildx and QEMU emulation in the CI pipeline.

## 3. Implementation Plan

### A. Update Dockerfiles

The existing `docker/backend.Dockerfile` and `docker/frontend.Dockerfile` will be updated to receive build-time arguments (`ARG`) and set labels (`LABEL`).

**Example additions:**

```dockerfile
ARG VERSION
ARG BUILD_DATE
ARG VCS_REF

LABEL org.opencontainers.image.version=$VERSION \
      org.opencontainers.image.created=$BUILD_DATE \
      org.opencontainers.image.revision=$VCS_REF \
      # ... other labels
```

### B. CI/CD Workflow (`.github/workflows/docker-publish.yml`)

We will create a new GitHub Actions workflow that:

1.  Triggers on push to `main` and on tags `v*`.
2.  Sets up QEMU (`docker/setup-qemu-action`) for multi-arch emulation.
3.  Sets up Docker Buildx (`docker/setup-buildx-action`).
4.  Logs in to GHCR.
5.  Extracts metadata (tags, labels) using `docker/metadata-action`.
6.  Builds and Pushes the images using `docker/build-push-action` with `platforms: linux/amd64,linux/arm64`.

### C. Registry

Images will be published to `ghcr.io/rustchat/rustchat-backend` and `ghcr.io/rustchat/rustchat-frontend`.

## 4. Local Development

A script `scripts/build_docker.sh` will be provided to allow developers to build the images locally. Note that building multi-arch locally can be slow due to emulation, so the local script might default to the host architecture unless specified otherwise.
