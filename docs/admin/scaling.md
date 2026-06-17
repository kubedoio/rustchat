# RustChat Scaling Guide

This guide covers the scalability features and configuration options for running RustChat in multi-node deployments.

## Table of Contents

1. [Multi-Node WebSocket Architecture](#multi-node-websocket-architecture)
2. [Cluster-Aware Connection Limits](#cluster-aware-connection-limits)
3. [Database Pool Tuning](#database-pool-tuning)
4. [Request Body Limits](#request-body-limits)
5. [Performance Monitoring](#performance-monitoring)

---

## Multi-Node WebSocket Architecture

### Overview

RustChat supports horizontal scaling with multiple server nodes using Redis as a message backbone for WebSocket event distribution.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Node 1    │     │   Node 2    │     │   Node 3    │
│  WS Hub     │◄───►│  WS Hub     │◄───►│  WS Hub     │
│  (Local)    │     │  (Local)    │     │  (Local)    │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │
                    ┌──────▼──────┐
                    │    Redis    │
                    │   Pub/Sub   │
                    │  (Cluster   │
                    │   Channel)  │
                    └─────────────┘
```

### How It Works

1. **Local Broadcast First**: Events are immediately broadcast to all connections on the local node
2. **Cluster Forward**: Events are published to Redis channel for other nodes
3. **Remote Receive**: Each node subscribes to the Redis channel and broadcasts received events locally
4. **Echo Prevention**: Messages include origin node ID to prevent loops

### Configuration

Multi-node mode is automatically enabled when Redis is configured. No additional configuration required.

```bash
# Ensure Redis is configured
RUSTCHAT_REDIS_URL=redis://redis-cluster:6379
```

### Monitoring Cluster Health

Nodes periodically send heartbeats to the cluster channel:

```
rustchat:cluster:ws:broadcast
```

Each heartbeat includes:
- Node ID
- Timestamp
- Local connection count

Monitor cluster health by subscribing to this channel:

```bash
redis-cli SUBSCRIBE rustchat:cluster:ws:broadcast
```

---

## Cluster-Aware Connection Limits

### Overview

Connection limits are enforced globally across all nodes using Redis, preventing users from exceeding limits by connecting to different nodes.

### Configuration

```bash
# Maximum connections per user (default: 5)
# This is stored in database server_config table, not env var
```

Set via API or database:

```sql
UPDATE server_config 
SET site = jsonb_set(site, '{max_simultaneous_connections}', '10')
WHERE id = 'default';
```

### How It Works

1. Each connection is registered in Redis with TTL
2. Connection count is checked against limit on new connection attempt
3. Stale connections are automatically cleaned up via Redis TTL

### Redis Keys

- `rustchat:presence:user:{user_id}:connections` - Set of active connection IDs
- `rustchat:conn:count:{user_id}` - Connection counter
- `rustchat:conn:heartbeat:{user_id}:{connection_id}` - Per-connection heartbeat

---

## Database Pool Tuning

### Configuration Options

```bash
# Maximum connections in the pool (default: 20)
DB_POOL_MAX_CONNECTIONS=50

# Minimum connections maintained (default: 5)
DB_POOL_MIN_CONNECTIONS=10

# Timeout to acquire a connection in seconds (default: 3)
DB_POOL_ACQUIRE_TIMEOUT=5

# Idle connection timeout in seconds (default: 600)
DB_POOL_IDLE_TIMEOUT=300

# Maximum connection lifetime in seconds (default: 1800)
DB_POOL_MAX_LIFETIME=3600
```

### Recommended Settings

| Deployment Size | MAX_CONNECTIONS | MIN_CONNECTIONS |
|----------------|-----------------|-----------------|
| Small (< 1k users) | 20 | 5 |
| Medium (1k-10k users) | 50 | 10 |
| Large (10k+ users) | 100+ | 20 |

### Connection Pool Formula

For PostgreSQL, ensure:
```
sum(all_node_pool_max) < PostgreSQL max_connections
```

Example:
- PostgreSQL max_connections = 200
- 4 RustChat nodes
- Max per node = 40 (4 × 40 = 160 < 200)

---

## Request Body Limits

### Overview

Route-specific body size limits prevent memory exhaustion from oversized requests while allowing necessary uploads.

### Default Limits

| Category | Size | Routes |
|----------|------|--------|
| Small | 64KB | Auth, status, simple queries |
| Medium | 1MB | Posts, user profiles, webhooks |
| Large | 50MB | File uploads, imports |

### Configuration

Body limits are currently hardcoded per route category. Future versions will support configuration.

### Custom Limits (Advanced)

To customize limits, modify `backend/src/api/mod.rs`:

```rust
const SMALL_BODY_LIMIT: usize = 64 * 1024;      // 64KB
const MEDIUM_BODY_LIMIT: usize = 1024 * 1024;   // 1MB
const LARGE_BODY_LIMIT: usize = 50 * 1024 * 1024; // 50MB
```

---

## Performance Monitoring

### Key Metrics

| Metric | Source | Alert Threshold |
|--------|--------|-----------------|
| DB Pool Saturation | SQLx metrics | > 80% |
| WS Connections/Node | Prometheus/Logs | Per-node limit |
| Redis Pub/Sub Lag | Redis monitoring | > 1s |
| Response Time p99 | APM/Tracing | > 500ms |

### Logging

Enable debug logging for scaling components:

```bash
RUST_LOG=rustchat::realtime=debug,rustchat::middleware=debug
```

### Health Checks

Health endpoint includes cluster status:

```bash
curl http://localhost:3000/api/v1/health
```

Response includes:
- Database connectivity
- Redis connectivity
- Connection counts

---

## Deployment Checklist

Before scaling to multiple nodes:

- [ ] Configure Redis for cluster pub/sub
- [ ] Tune database pool sizes
- [ ] Set appropriate connection limits
- [ ] Configure load balancer with sticky sessions (optional)
- [ ] Set up monitoring for cluster health
- [ ] Test WebSocket failover behavior

### Load Balancer Configuration

For WebSocket support, ensure your load balancer:

1. **Supports WebSocket upgrades**
2. **Forwards client IP** (`X-Forwarded-For` header)
3. **Optional: Sticky sessions** (reduces cross-node broadcasts)

Example nginx configuration:

```nginx
upstream rustchat {
    server node1:3000;
    server node2:3000;
    server node3:3000;
}

server {
    listen 443 ssl;
    
    location / {
        proxy_pass http://rustchat;
        proxy_http_version 1.1;
        
        # WebSocket support
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        
        # Forward client info
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header Host $host;
    }
}
```

---

## Troubleshooting

### WebSocket Events Not Reaching All Users

**Symptoms**: Messages sent by user A aren't received by user B on a different node.

**Checks**:
1. Verify Redis connectivity on all nodes
2. Check cluster subscriber is running:
   ```bash
   redis-cli PUBSUB CHANNELS | grep rustchat
   ```
3. Look for errors in logs:
   ```
   "Cluster subscriber error"
   "Failed to broadcast to cluster"
   ```

### Connection Limit Issues

**Symptoms**: Users can't connect despite being under the limit.

**Checks**:
1. Verify Redis has accurate counts:
   ```bash
   redis-cli SCARD rustchat:presence:user:{user_id}:connections
   ```
2. Check for stale entries (should auto-expire via TTL)
3. Review connection limit setting in database

### Database Pool Exhaustion

**Symptoms**: Requests timing out, "pool timed out" errors.

**Checks**:
1. Monitor pool saturation:
   ```sql
   SELECT count(*) FROM pg_stat_activity 
   WHERE application_name = 'rustchat';
   ```
2. Increase pool size or add nodes
3. Check for slow queries causing pool starvation

---

## Migration from Single Node

To migrate from single-node to multi-node:

1. **Prepare Redis** (if not already configured)
2. **Deploy additional nodes** with same configuration
3. **Verify cluster communication** via logs
4. **Update load balancer** to include new nodes
5. **Monitor connection distribution**

No data migration required - all state is in PostgreSQL/Redis.
