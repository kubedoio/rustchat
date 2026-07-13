# Build stage
FROM node:24-alpine@sha256:a0b9bf06e4e6193cf7a0f58816cc935ff8c2a908f81e6f1a95432d679c54fbfd AS builder
RUN apk add --no-cache git
WORKDIR /app
COPY package.json package-lock.json .npmrc dependency-policy.json dependency-patches.json ./
COPY scripts ./scripts
COPY patches ./patches
RUN node scripts/check-dependency-policy.mjs
RUN npm ci --ignore-scripts
RUN node scripts/apply-dependency-patches.mjs
COPY . .
RUN npm run build

# Production stage
FROM openresty/openresty:alpine@sha256:99b32fe3e411c98033114dd471440fb702992d0953ce8b6e6b5c016285ac2ab9

# Create a dedicated non-root user and group for running the frontend server.
RUN addgroup -S rustchat && adduser -S rustchat -G rustchat

# Create required directories and ensure the non-root user owns the paths
# nginx needs to write to (logs, pid file, temp directories).
RUN mkdir -p /var/log/nginx /tmp/nginx /usr/share/nginx/html && \
    chown -R rustchat:rustchat /var/log/nginx /tmp/nginx /usr/share/nginx/html

# Copy built assets with non-root ownership.
COPY --from=builder --chown=rustchat:rustchat /app/dist /usr/share/nginx/html

# Copy the complete nginx main configuration (includes listen 8080 and pid /tmp/nginx.pid).
COPY --chown=rustchat:rustchat nginx.conf /usr/local/openresty/nginx/conf/nginx.conf

EXPOSE 8080

USER rustchat

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:8080/ || exit 1

CMD ["/usr/local/openresty/bin/openresty", "-g", "daemon off;"]
