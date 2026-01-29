# Build stage
FROM rust:1.93-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

# Ensure portable builds (avoid CPU-specific instructions)
ENV RUSTFLAGS="-C target-cpu=x86-64"

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy src for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Build the application
ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.20

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

RUN apk add --no-cache ca-certificates libgcc

WORKDIR /app

# Copy the binary
COPY --from=builder /app/target/release/rustchat /usr/local/bin/rustchat

# Copy migrations for runtime
COPY --from=builder /app/migrations ./migrations

# Create non-root user
RUN adduser -D -u 1000 rustchat
USER rustchat

EXPOSE 3000

ENV RUST_LOG=info

CMD ["rustchat"]
