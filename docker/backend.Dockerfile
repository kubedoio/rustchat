# Build stage
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy src for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source
# Copy actual source
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Build the application
ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.20

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
