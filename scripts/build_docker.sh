#!/bin/bash
set -e

# Configuration
VERSION=$(git describe --tags --always --dirty || echo "dev")
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
VCS_REF=$(git rev-parse HEAD || echo "unknown")
REPO="rustchat"

echo "Building Docker images for version: $VERSION"
echo "Build Date: $BUILD_DATE"
echo "VCS Ref: $VCS_REF"

# Function to build an image
build_image() {
    local SERVICE=$1
    local DOCKERFILE=$2
    local CONTEXT=$3
    local IMAGE_NAME="$REPO-$SERVICE"

    echo "----------------------------------------------------------------"
    echo "Building $SERVICE..."
    echo "----------------------------------------------------------------"

    docker build \
        --build-arg VERSION="$VERSION" \
        --build-arg BUILD_DATE="$BUILD_DATE" \
        --build-arg VCS_REF="$VCS_REF" \
        -t "$IMAGE_NAME:$VERSION" \
        -t "$IMAGE_NAME:latest" \
        -f "$DOCKERFILE" \
        "$CONTEXT"

    echo "Successfully built $IMAGE_NAME:$VERSION"
}

# Build Backend
build_image "backend" "docker/backend.Dockerfile" "backend"

# Build Frontend
build_image "frontend" "docker/frontend.Dockerfile" "frontend"

echo "----------------------------------------------------------------"
echo "All images built successfully!"
echo "----------------------------------------------------------------"
echo "List of images:"
docker images | grep "$REPO"
