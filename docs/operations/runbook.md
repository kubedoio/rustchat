# RustChat Operations Runbook

This runbook provides procedures for common operational tasks and incident response.

## Table of Contents

1. [Deployment](#deployment)
2. [Monitoring](#monitoring)
3. [Incident Response](#incident-response)
4. [Maintenance](#maintenance)

---

## Deployment

### First-Time Deployment

```bash
# 1. Generate secrets
export RUSTCHAT_JWT_SECRET="$(openssl rand -base64 48)"
export RUSTCHAT_ENCRYPTION_KEY="$(openssl rand -base64 48)"

# 2. Create environment file
cat > .env << EOF
RUSTCHAT_ENVIRONMENT=production
RUSTCHAT_JWT_SECRET=${RUSTCHAT_JWT_SECRET}
RUSTCHAT_ENCRYPTION_KEY=${RUSTCHAT_ENCRYPTION_KEY}
RUSTCHAT_DATABASE_URL=postgres://user:pass@db:5432/rustchat
RUSTCHAT_REDIS_URL=redis://redis:6379
RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie
EOF

# 3. Verify configuration
./rustchat --check-config

# 4. Run database migrations
./rustchat --migrate-only

# 5. Start service
./rustchat
```

### Rolling Update (Zero-Downtime)

```bash
# 1. Verify health of existing nodes
for node in node1 node2 node3; do
  curl -sf http://${node}:3000/api/v1/health/ready || exit 1
done

# 2. Update nodes one at a time
for node in node1 node2 node3; do
  echo "Updating ${node}..."
  
  # Drain connections (if using a load balancer)
  curl -X POST http://lb/admin/drain/${node}
  
  # Wait for connections to drain
  sleep 30
  
  # Deploy new version
  ssh ${node} "systemctl restart rustchat"
  
  # Wait for health check
  until curl -sf http://${node}:3000/api/v1/health/ready; do
    sleep 5
  done
  
  # Re-enable in load balancer
  curl -X POST http://lb/admin/enable/${node}
done
```

---

## Monitoring

### Key Metrics

| Metric | Query | Alert Threshold |
|--------|-------|-----------------|
| Error Rate | `rate(rustchat_http_requests_total{status=~"5.."}[5m])` | > 1% |
| Latency p99 | `histogram_quantile(0.99, rate(rustchat_http_request_duration_seconds_bucket[5m]))` | > 500ms |
| DB Pool Saturation | `rustchat_db_pool_saturation_ratio` | > 0.8 |
| WS Connections | `rustchat_websocket_active_connections` | Per-node limit |
| Circuit Breaker | `rustchat_circuit_breaker_state_changes_total` | Any state change |
| RAG Indexing Duration | `rustchat_rag_indexing_duration_seconds` | Sustained increase |
| RAG Search Duration | `rustchat_rag_search_duration_seconds` | p99 > 500ms |

### Health Checks

```bash
# Liveness (Kubernetes)
curl http://localhost:3000/api/v1/health/live
# Expected: {"status":"ok","version":"0.3.1","uptime_seconds":3600}

# Readiness (Kubernetes)
curl http://localhost:3000/api/v1/health/ready
# Expected: 200 OK with all checks passing

# Metrics (Prometheus)
curl http://localhost:3000/api/v1/health/metrics
# Expected: Prometheus exposition format

# Stats (JSON)
curl http://localhost:3000/api/v1/health/stats
# Expected: {"websocket_connections":150,"active_users":42,"db_pool_saturation":0.3}
```

### Log Queries

```bash
# Error spikes
kubectl logs -l app=rustchat | jq 'select(.level=="ERROR")'

# Slow queries
kubectl logs -l app=rustchat | jq 'select(.fields.latency_ms > 1000)'

# Authentication failures
kubectl logs -l app=rustchat | jq 'select(.message | contains("Unauthorized"))'

# WebSocket events
kubectl logs -l app=rustchat | jq 'select(.span.name=="ws_handler")'

# Agent runtime events
kubectl logs -l app=rustchat | jq 'select(.target | contains("agent"))'

# RAG indexing/search events
kubectl logs -l app=rustchat | jq 'select(.target | contains("knowledge"))'
```

---

## Incident Response

### P1: Service Down

**Symptoms**: Health checks failing, 0% availability

```bash
# 1. Check if process is running
systemctl status rustchat

# 2. Check logs for startup errors
journalctl -u rustchat -n 100 --no-pager

# 3. Verify database connectivity
psql $RUSTCHAT_DATABASE_URL -c "SELECT 1"

# 4. Verify Redis connectivity
redis-cli -u $RUSTCHAT_REDIS_URL PING

# 5. Check port binding
ss -tlnp | grep 3000

# 6. Emergency restart
systemctl restart rustchat
```

### P2: High Error Rate

**Symptoms**: > 5% 5xx errors

```bash
# 1. Identify error types
kubectl logs -l app=rustchat | jq -r '.message' | sort | uniq -c | sort -rn | head -20

# 2. Check database performance
# Look for slow queries in PostgreSQL logs

# 3. Check Redis latency
redis-cli --latency -h redis

# 4. Check circuit breaker states
curl http://localhost:3000/api/v1/health/stats | jq '.circuit_breakers'

# 5. Scale up if needed
kubectl scale deployment rustchat --replicas=6
```

### P3: Database Connection Exhaustion

**Symptoms**: "pool timed out" errors

```bash
# 1. Check current connections
psql -c "SELECT count(*) FROM pg_stat_activity WHERE application_name='rustchat';"

# 2. Check for idle connections
psql -c "SELECT pid, state, query_start, query FROM pg_stat_activity WHERE state='idle in transaction';"

# 3. Kill idle transactions (carefully!)
psql -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state='idle in transaction' AND age(now(), query_start) > interval '5 minutes';"

# 4. Temporary fix: Increase pool size
export RUSTCHAT_DB_POOL__MAX_CONNECTIONS=100
systemctl restart rustchat

# 5. Permanent fix: Check for connection leaks in code
```

### P4: WebSocket Connection Issues

**Symptoms**: Users reporting real-time updates not working

```bash
# 1. Check WebSocket connection count
curl http://localhost:3000/api/v1/health/stats | jq '.websocket_connections'

# 2. Check for connection limit violations
kubectl logs -l app=rustchat | grep "Too many connections"

# 3. Check Redis pub/sub
redis-cli PUBSUB CHANNELS | grep rustchat

# 4. Check cluster heartbeat
kubectl logs -l app=rustchat | grep "cluster heartbeat"

# 5. Test WebSocket directly
websocat ws://localhost:3000/api/v1/ws -H "Authorization: Bearer TOKEN"
```

### P5: OAuth Authentication Failure

**Symptoms**: Users can't log in via OAuth

```bash
# 1. Check circuit breaker state for OIDC
kubectl logs -l app=rustchat | grep "circuit.*oidc"

# 2. Test OIDC discovery
curl https://accounts.google.com/.well-known/openid-configuration | jq '.authorization_endpoint'

# 3. Check OAuth callback URL configuration
# Verify RUSTCHAT_SITE_URL matches registered redirect URI

# 4. Check for state parameter mismatches
kubectl logs -l app=rustchat | grep "OAuth state"

# 5. Reset circuit breaker (emergency)
# Restart affected node or wait for auto-recovery
```

### P6: Rate Limiting Too Aggressive

**Symptoms**: Legitimate users getting 429 errors

```bash
# 1. Check rate limit metrics
curl http://localhost:3000/api/v1/health/metrics | grep rate_limit

# 2. Temporarily increase limits
export RUSTCHAT_SECURITY_RATE_LIMIT_AUTH_PER_MINUTE=20
export RUSTCHAT_SECURITY_RATE_LIMIT_WS_PER_MINUTE=60
systemctl restart rustchat

# 3. Identify source of high traffic
kubectl logs -l app=rustchat | grep "Rate limit exceeded" | jq '.ip' | sort | uniq -c | sort -rn | head -10
```

### P7: AI Agents Not Responding

**Symptoms**: Users mention an agent but no response appears

```bash
# 1. Confirm the runtime registered a provider
kubectl logs -l app=rustchat | grep -E "OpenAI LLM provider registered|agent runtime disabled"

# 2. Check provider and tool environment
kubectl exec deploy/rustchat -- printenv | grep -E 'RUSTCHAT_OPENAI_API_KEY|OPENAI_API_KEY|TAVILY_API_KEY'

# 3. Check for LLM errors or rate limits
kubectl logs -l app=rustchat | grep -Ei "agent|llm|rate limit|provider"

# 4. Verify the agent is assigned to the channel in the admin UI
# Admin Console > AI Agents > Agent detail > Channels

# 5. Restart after adding provider keys
kubectl rollout restart deployment/rustchat
```

If the backend logs `No LLM providers configured; agent runtime disabled`, configure `RUSTCHAT_OPENAI_API_KEY` or `OPENAI_API_KEY` and restart the backend.

### P8: Knowledge Base Indexing Failure

**Symptoms**: Agent answers ignore uploaded documents, or knowledge base status remains unindexed

```bash
# 1. Check pgvector availability
psql $RUSTCHAT_DATABASE_URL -c "SELECT extname FROM pg_extension WHERE extname = 'vector';"

# 2. Check RAG metrics
curl http://localhost:3000/api/v1/health/metrics | grep rag

# 3. Check backend logs for ingestion failures
kubectl logs -l app=rustchat | grep -Ei "knowledge|embedding|vector|sync source"

# 4. Verify S3 access from backend configuration
kubectl logs -l app=rustchat | grep -Ei "s3|storage"

# 5. Retry sync from Admin Console > Knowledge Bases
```

Large RustShare folders can take seconds to minutes to index. If indexing repeatedly fails, verify S3 credentials, provider key validity, outbound network access, and the `pgvector` extension.

### P9: Agent Cost or Usage Spike

**Symptoms**: Provider usage increases suddenly, responses appear too often, or channels receive too many agent posts

```bash
# 1. Check agent-related logs
kubectl logs -l app=rustchat --since=1h | grep -Ei "agent|tokens|usage"

# 2. Temporarily reduce agent limits
export RUSTCHAT_AGENT_MAX_REQUESTS_PER_MINUTE=5
export RUSTCHAT_AGENT_MAX_TOKENS_PER_HOUR=25000
systemctl restart rustchat

# 3. Disable optional web search tool if needed
unset TAVILY_API_KEY
systemctl restart rustchat
```

Review agents with `respond_to_all` enabled, broad channel assignments, and large knowledge base assignments. Disable or narrow the agent in the admin UI before increasing provider limits.

---

## Maintenance

### Secret Rotation

```bash
# 1. Generate new secret
NEW_JWT_SECRET="$(openssl rand -base64 48)"

# 2. Deploy with new secret (rolling update)
# Users will be logged out as tokens become invalid

# 3. Monitor for auth errors
watch 'kubectl logs -l app=rustchat | grep -c "Invalid token"'

# 4. Users will naturally re-authenticate
```

### Database Maintenance

```bash
# 1. Check table bloat
psql -c "SELECT schemaname, tablename, n_tup_ins, n_tup_upd, n_tup_del FROM pg_stat_user_tables ORDER BY n_tup_del DESC;"

# 2. Vacuum analyze
psql -c "VACUUM ANALYZE;"

# 3. Check index usage
psql -c "SELECT schemaname, tablename, indexname, idx_scan FROM pg_stat_user_indexes ORDER BY idx_scan DESC;"

# 4. Reindex if needed
psql -c "REINDEX INDEX CONCURRENTLY idx_name;"
```

### Redis Maintenance

```bash
# 1. Check memory usage
redis-cli INFO memory

# 2. Check key count
redis-cli DBSIZE

# 3. Find large keys
redis-cli --bigkeys

# 4. Expire old keys manually (if needed)
redis-cli EVAL "return redis.call('del', unpack(redis.call('keys', 'rustchat:oauth:code:*')))" 0
```

### Backup Procedures

```bash
# Database backup
pg_dump $RUSTCHAT_DATABASE_URL | gzip > rustchat_backup_$(date +%Y%m%d).sql.gz

# Redis backup (if using persistence)
redis-cli BGSAVE

# Configuration backup
cp .env .env.backup_$(date +%Y%m%d)
```

### AI Agent Maintenance

```bash
# Confirm optional runtime configuration
grep -E 'RUSTCHAT_OPENAI_API_KEY|OPENAI_API_KEY|TAVILY_API_KEY|RUSTCHAT_AGENT_' .env

# Confirm pgvector for RAG
psql $RUSTCHAT_DATABASE_URL -c "CREATE EXTENSION IF NOT EXISTS vector;"

# Review recent agent runtime errors
kubectl logs -l app=rustchat --since=24h | grep -Ei "agent|llm|knowledge|embedding|tool"
```

Maintenance checklist:
- Rotate LLM provider and tool keys with the regular secret rotation schedule.
- Review agent channel assignments after team or permission changes.
- Review knowledge base assignments after new sensitive documents are added.
- Confirm backups include both PostgreSQL and the S3 bucket that stores knowledge documents.

---

## Performance Tuning

### Database Optimization

```sql
-- Add index for common queries if missing
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_posts_channel_created 
ON posts(channel_id, created_at DESC);

-- Check for slow queries
SELECT query, mean_exec_time, calls 
FROM pg_stat_statements 
ORDER BY mean_exec_time DESC 
LIMIT 10;
```

### Connection Pool Tuning

```bash
# Formula: max_connections = (CPU cores * 2) + effective_spindle_count
# For cloud: max_connections = vCPU * 4

# Example for 4 vCPU instance
export RUSTCHAT_DB_POOL__MAX_CONNECTIONS=16
export RUSTCHAT_DB_POOL__MIN_CONNECTIONS=4
```

### WebSocket Optimization

```bash
# Increase per-node connection limit (if memory allows)
# Update in database:
UPDATE server_config 
SET site = jsonb_set(site, '{max_simultaneous_connections}', '10')
WHERE id = 'default';

# Scale horizontally instead of vertically
kubectl scale deployment rustchat --replicas=10
```

---

## Security Incident Response

### Suspected Breach

1. **Immediate Actions**:
   ```bash
   # Rotate all secrets immediately
   kubectl create secret generic rustchat-secrets --from-literal=jwt=$(openssl rand -base64 48) --dry-run -o yaml | kubectl apply -f -
   kubectl rollout restart deployment/rustchat
   ```

2. **Audit**:
   ```bash
   # Check auth logs
   kubectl logs -l app=rustchat --since=24h | jq 'select(.message | contains("auth"))'
   
   # Check for unusual access patterns
   kubectl logs -l app=rustchat | jq 'select(.fields.status | tonumber > 400)'
   ```

3. **Notify**:
   - Security team
   - Users (if data affected)

---

## Contact & Escalation

| Severity | Response Time | Contact |
|----------|--------------|---------|
| P1 (Service Down) | 15 minutes | On-call engineer |
| P2 (Degraded) | 1 hour | Engineering team |
| P3 (Minor) | 4 hours | Engineering team |
| Security | Immediate | Security team |
