# Build stage
FROM node:26-alpine AS builder
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
FROM openresty/openresty:alpine

# Create required directories
RUN mkdir -p /var/log/nginx /var/run/openresty

# Copy built assets
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080

CMD ["/usr/local/openresty/bin/openresty", "-g", "daemon off;"]
