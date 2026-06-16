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

# Create required directories
RUN mkdir -p /var/log/nginx /var/run/openresty

# Copy built assets
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:8080/ || exit 1

CMD ["/usr/local/openresty/bin/openresty", "-g", "daemon off;"]
