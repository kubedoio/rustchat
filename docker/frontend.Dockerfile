# Build stage
FROM node:24-alpine@sha256:fb71d01345f11b708a3553c66e7c74074f2d506400ea81973343d915cb64eef0 AS builder
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
FROM openresty/openresty:alpine@sha256:49db7235f2f94aa179c1242882619aea258c112b20f48ba45aefba010a1d0607

# Create a dedicated non-root user and group for running the frontend server.
RUN addgroup -S rustchat && adduser -S rustchat -G rustchat

# Create required directories and ensure the non-root user owns the paths
# nginx needs to write to (logs, pid file, temp directories).
RUN mkdir -p /var/log/nginx /var/run/openresty /tmp/nginx /usr/local/openresty/nginx/logs /usr/share/nginx/html && \
    chown -R rustchat:rustchat /var/log/nginx /var/run/openresty /tmp/nginx /usr/local/openresty/nginx/logs /usr/share/nginx/html

# Copy built assets with non-root ownership.
COPY --from=builder --chown=rustchat:rustchat /app/dist /usr/share/nginx/html

# Copy the complete nginx main configuration (includes listen 8080 and pid /tmp/nginx.pid).
COPY --chown=rustchat:rustchat nginx.conf /usr/local/openresty/nginx/conf/nginx.conf

EXPOSE 8080

USER rustchat

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:8080/ || exit 1

CMD ["/usr/local/openresty/bin/openresty", "-g", "daemon off;"]
