# Build stage
FROM rust:1.97-alpine@sha256:3c38f3f82c2f3d73da3b38e18d279393a04cb43ddded0e35088a8c3324d40900 AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

# Keep builds architecture-neutral across amd64/arm64 hosts.
ENV RUSTFLAGS="-C target-cpu=generic"

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

FROM alpine:3.24@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b AS ci-validate

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN test -f Cargo.toml && test -f Cargo.lock && test -f src/main.rs

FROM builder AS app-builder

ARG TARGETARCH

# Create dummy src for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN --mount=type=cache,id=backend-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=backend-cargo-target-${TARGETARCH},target=/app/target \
    cargo build --release --locked && \
    rm -rf src

# Copy actual source
COPY src ./src
COPY migrations ./migrations
# COPY .sqlx ./.sqlx

# Build with cache mounts for faster rebuilds
# BuildKit caches cargo registry and build artifacts between builds
ENV SQLX_OFFLINE=true
RUN --mount=type=cache,id=backend-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=backend-cargo-target-${TARGETARCH},target=/app/target \
    touch src/main.rs && \
    cargo build --release --locked && \
    cp /app/target/release/rustchat /tmp/rustchat

# Runtime stage
FROM alpine:3.24@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b

ARG VERSION
ARG BUILD_DATE
ARG VCS_REF

LABEL org.opencontainers.image.title="rustchat-backend" \
      org.opencontainers.image.description="Rustchat Backend Server" \
      org.opencontainers.image.source="https://github.com/rustchat/rustchat" \
      org.opencontainers.image.version=$VERSION \
      org.opencontainers.image.created=$BUILD_DATE \
      org.opencontainers.image.revision=$VCS_REF \
      org.opencontainers.image.licenses="MIT"

RUN apk add --no-cache ca-certificates libgcc wget

WORKDIR /app

# Copy the binary (from tmp since target is a cache mount)
COPY --from=app-builder /tmp/rustchat /usr/local/bin/rustchat

# Copy migrations for runtime
COPY --from=app-builder /app/migrations ./migrations

# Create non-root user
RUN adduser -D -u 1000 rustchat
USER rustchat

EXPOSE 3000

ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:3000/api/v1/health/live || exit 1

CMD ["rustchat"]
