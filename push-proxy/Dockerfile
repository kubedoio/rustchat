FROM lukemathwalker/cargo-chef:latest-rust-1@sha256:e606721f52d95169364bf39cae726a94ed8b397625011ccfaa8340db488b823b AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin push-proxy

# We use debian:bookworm-slim for the runtime image
FROM debian:bookworm-slim@sha256:96e378d7e6531ac9a15ad505478fcc2e69f371b10f5cdf87857c4b8188404716 AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl3 wget && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/push-proxy /usr/local/bin

# Run as non-root user
RUN useradd -m -u 1000 pushproxy
USER pushproxy

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:3000/health 2>&1 | grep -q 'ok' || exit 1

ENTRYPOINT ["/usr/local/bin/push-proxy"]
