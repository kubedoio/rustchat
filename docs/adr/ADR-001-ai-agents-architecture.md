# ADR-001: AI Agents as Channel Participants

**Date:** 2026-06-09
**Status:** Proposed
**Risk tier:** architectural
**Issue:** [#177](https://github.com/kubedoio/rustchat/issues/177)

## Context

RustChat currently supports four entity types: `Human`, `Agent`, `Service`, and `CI`. The `Agent` type exists in the data model, API key authentication works, and rate limiting is configured. However, there is no actual AI runtime: the `/api/v4/ai/*` endpoints return empty arrays, agents cannot participate in channels, and there is no mechanism for an agent to respond when mentioned.

The requirement is to make agents first-class channel participants: an admin creates an agent (title, profile, AI token, system prompt, memory/RAG config), assigns it to channels, and when a user pings the agent with `@agent-name`, the agent reads channel context, calls an LLM, and posts a reply.

This touches auth/permissions (agent capabilities), storage model (new tables), API contracts (new endpoints), and the real-time message pipeline (WebSocket broadcast). It is an architectural-tier change per `.governance/risk-tiers.yml`.

---

## Decision

We will build an **Agent Runtime** that leverages the existing entity system rather than replacing it. The architecture has five layers:

### 1. Identity Layer (Reuse Existing)

Agents are `users` rows with `entity_type = 'agent'`. We reuse:

- `users.id`, `users.username`, `users.display_name`, `users.avatar_url` — public identity
- `users.entity_type = 'agent'` — discriminator
- `users.api_key_hash` / `users.api_key_prefix` — authentication for agent-to-agent or external calls
- `users.role = 'member'` or `'guest'` — RBAC inside channels
- `channel_members` — standard channel membership (no new join/leave mechanics)
- Existing mention parser (`@username`) — no new syntax

### 2. Configuration Layer (New Tables)

`entity_metadata` JSONB is too loose for structured queries and validation. We introduce two new tables:

#### `agent_configs`

```sql
CREATE TABLE agent_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    -- Identity & behavior
    title VARCHAR(128) NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL DEFAULT '',
    -- LLM provider settings
    provider VARCHAR(32) NOT NULL DEFAULT 'openai',   -- openai, anthropic, ollama, etc.
    model VARCHAR(64) NOT NULL DEFAULT 'gpt-4o-mini',
    api_token_encrypted TEXT,                          -- encrypted LLM API key
    temperature REAL NOT NULL DEFAULT 0.7 CHECK (temperature BETWEEN 0.0 AND 2.0),
    max_context_messages INT NOT NULL DEFAULT 20 CHECK (max_context_messages BETWEEN 1 AND 100),
    max_output_tokens INT NOT NULL DEFAULT 1024 CHECK (max_output_tokens BETWEEN 1 AND 8192),
    -- Capabilities (bitmask or JSON)
    capabilities JSONB NOT NULL DEFAULT '{"respond_to_mentions": true, "respond_to_all": false, "use_memory": true, "use_rag": false}',
    -- RAG settings (optional)
    rag_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    rag_top_k INT NOT NULL DEFAULT 5 CHECK (rag_top_k BETWEEN 1 AND 20),
    -- State
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id)
);
```

#### `agent_memories`

```sql
CREATE TABLE agent_memories (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent_configs(user_id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    -- Memory content
    memory_type VARCHAR(32) NOT NULL DEFAULT 'conversation',  -- conversation, fact, preference
    content TEXT NOT NULL,
    -- Context for retrieval
    message_ids UUID[],                                         -- which messages this memory relates to
    embedding VECTOR(1536),                                     -- pgvector for semantic search (Phase 2)
    -- Metadata
    importance_score REAL NOT NULL DEFAULT 1.0 CHECK (importance_score BETWEEN 0.0 AND 1.0),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    CONSTRAINT agent_memories_agent_channel UNIQUE (agent_id, channel_id, memory_type, content)
);
CREATE INDEX idx_agent_memories_agent_channel ON agent_memories(agent_id, channel_id);
CREATE INDEX idx_agent_memories_embedding ON agent_memories USING ivfflat (embedding vector_cosine_ops);  -- Phase 2
```

**Why not JSONB in `users`?**
- We need to query `SELECT * FROM agent_configs WHERE provider = 'openai'` for admin dashboards.
- We need foreign-key constraints and cascading deletes when an agent user is deleted.
- Typed columns prevent invalid config at the DB level (CHECK constraints on temperature, tokens).

### 3. LLM Provider Layer (New Trait + Implementations)

A trait-based provider system keeps the agent runtime provider-agnostic and testable:

```rust
// backend/src/services/llm/mod.rs
pub trait LlmProvider: Send + Sync {
    /// Provider name for metrics/logging
    fn name(&self) -> &'static str;
    
    /// Send a conversation to the LLM and return the text response
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    
    /// (Optional) Stream tokens for real-time response preview
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>, LlmError>;
}

pub struct CompletionRequest {
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,      // role + content
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

pub struct ChatMessage {
    pub role: MessageRole,               // System, User, Assistant
    pub content: String,
    pub name: Option<String>,            // for distinguishing multiple users
}
```

**Initial providers:**
- `OpenAiProvider` — OpenAI GPT-4o, GPT-4o-mini, o1, etc.
- `AnthropicProvider` — Claude 3.5 Sonnet, Opus, etc.
- `OllamaProvider` — Local/self-hosted models via Ollama API

**Provider registry:**
```rust
// backend/src/services/llm/registry.rs
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
}
```

The registry is initialized at startup from environment variables (e.g., `RUSTCHAT_OPENAI_API_KEY`, `RUSTCHAT_ANTHROPIC_API_KEY`). Each provider is behind an `Arc` so multiple agents can share the same HTTP client connection pool.

**API token encryption:**
- Agent LLM API tokens are encrypted at rest using AES-256-GCM with the app's `ENCRYPTION_KEY`.
- Decryption happens only inside the `LlmProvider` implementation just before the HTTP request.
- This prevents a DB dump from leaking LLM provider credentials.

### 4. Agent Runtime Layer (New Service)

The runtime is a background service that detects when an agent should respond and orchestrates the full pipeline:

```rust
// backend/src/services/agent_runtime.rs
pub struct AgentRuntime {
    db: PgPool,
    ws_hub: Arc<WsHub>,
    provider_registry: Arc<ProviderRegistry>,
    post_service: Arc<PostService>,
}
```

**Trigger conditions:**
1. **Mention trigger** (`@agent-username` in a message) — primary
2. **All-messages trigger** (agent configured with `respond_to_all: true`) — opt-in per agent
3. **Keyword trigger** (future: `/agent-name: do something`)

**Response pipeline (per trigger):**

```
1. RECEIVE trigger (WebSocket event or REST post creation)
   ↓
2. FETCH agent config (agent_configs + user row)
   ↓
3. CHECK permissions (is agent active? is agent member of channel?)
   ↓
4. BUILD context
   ├── Fetch last N messages from channel (respecting max_context_messages)
   ├── Fetch agent memories for this channel
   ├── (Phase 2) RAG: semantic search over channel history via pgvector
   └── Format into ChatMessage[] with system_prompt prepended
   ↓
5. CALL LLM (via provider registry, with timeout + retry)
   ↓
6. PARSE & VALIDATE response (strip markdown if needed, length check)
   ↓
7. CREATE post (as the agent user, via PostService)
   ├── Insert into posts table
   ├── Attach file_ids if any
   └── Broadcast via WebSocket hub
   ↓
8. STORE memory (summarize conversation turn, upsert into agent_memories)
```

**Concurrency model:**
- Each trigger spawns a **detached Tokio task** (`tokio::spawn`) so the HTTP/WebSocket handler returns immediately.
- A per-agent **semaphore** (limit = 1) prevents concurrent responses from the same agent in the same channel. If an agent is already generating a response, subsequent mentions queue or get a "thinking..." status.
- A global **rate limiter** (Redis-backed, existing) caps agent LLM calls per minute.

**Error handling:**
- LLM timeout (>30s) → log error, do not post.
- LLM API error (rate limit, auth failure) → log error, optionally post a brief "I'm having trouble" message.
- Agent config not found → silently drop (should not happen).
- Agent not member of channel → silently drop (security).

### 5. Admin & Frontend Layer

#### Backend API (New)

```
GET    /api/v1/agents              → list agents (admin only)
POST   /api/v1/agents              → create agent (admin only)
GET    /api/v1/agents/:id          → get agent config
PUT    /api/v1/agents/:id          → update agent config (admin or creator)
DELETE /api/v1/agents/:id          → delete agent (soft delete user + cascade config)
POST   /api/v1/agents/:id/regenerate-key → rotate API key

GET    /api/v1/agents/:id/channels → list channels agent is in
POST   /api/v1/agents/:id/channels/:channel_id → add agent to channel
DELETE /api/v1/agents/:id/channels/:channel_id → remove agent from channel

GET    /api/v1/agents/:id/memories → list memories (admin or creator)
DELETE /api/v1/agents/:id/memories/:memory_id → delete memory

POST   /api/v1/agents/:id/test     → test prompt with a sample message (admin only)
```

All endpoints use `AuthUser` extractor and the existing `PolicyEngine` for authorization.

#### Frontend (New Views + Components)

**Admin Console:**
- `/admin/agents` — AgentManagement view (table with search, status, provider, model)
- CreateAgentModal — form with tabs: Basic (name, username, avatar), LLM (provider, model, token, temperature), Behavior (system prompt, capabilities, channel selection), Memory (RAG toggle, max context)
- EditAgentModal — same form, pre-populated
- AgentChannelPicker — multi-select dropdown of channels to assign agent to

**Channel UI:**
- MessageItem already renders `@username` highlights. No change needed.
- ChannelSidebar shows agent members alongside human members (filtered by `entity_type = 'agent'`).
- TypingIndicator support for agents (shows "Code Assistant is typing..." when agent is generating).

---

## Consequences

### Positive

- **Leverages existing infrastructure** — entity system, API keys, WebSocket hub, mention parser, channel membership, RBAC. No rewrites.
- **Provider-agnostic** — teams can use OpenAI, Anthropic, or local Ollama. Easy to add new providers.
- **Memory is opt-in per agent** — agents without memory act as stateless assistants, keeping resource usage low.
- **Encrypted LLM tokens** — API keys are not stored in plaintext even if the DB is compromised.
- **Familiar UX** — `@agent-name` works exactly like `@human-name`. Agents appear in member lists.

### Negative / Trade-offs

- **LLM call latency** — every mention incurs an LLM round-trip (500ms–5s). Users see a delay. Mitigation: typing indicator + streaming tokens (Phase 2).
- **Cost exposure** — if `respond_to_all` is enabled, every message in a channel triggers an LLM call. Mitigation: `respond_to_all` defaults to `false`; rate limiting; admin approval required.
- **Memory storage grows unbounded** — `agent_memories` accumulates over time. Mitigation: `importance_score` + TTL (`expires_at`) + background cleanup job.
- **pgvector dependency** (Phase 2) — requires `pgvector` PostgreSQL extension. Not all managed DBs support it. Mitigation: RAG is optional; without pgvector, memory uses exact-match retrieval only.
- **No streaming in Phase 1** — the entire LLM response is fetched before posting. Mitigation: Phase 2 adds `complete_stream` + progressive WebSocket messages.

### Security

- Agent LLM API tokens are encrypted at rest (AES-256-GCM).
- Agents cannot read channels they are not members of (enforced at DB query level via `channel_members` join).
- Agent responses go through the same `PostService` validation as human posts (length limits, file validation, mention parsing).
- `respond_to_all` requires `system_admin` or `org_admin` approval to enable.
- Agent system prompts are not exposed to non-admin users via API.

### Performance

- LLM calls are **async detached tasks** — they never block HTTP/WebSocket handlers.
- Context building queries are indexed (`posts(channel_id, created_at)` already exists).
- Memory queries use `(agent_id, channel_id)` index.
- Per-agent semaphore prevents thundering herd if a channel has many simultaneous mentions.

---

## Rollout Plan

### Phase 1: Foundation (MVP)
- [ ] Migration: `agent_configs`, `agent_memories` tables
- [ ] `LlmProvider` trait + `OpenAiProvider` implementation
- [ ] `AgentRuntime` service with mention trigger
- [ ] `POST /api/v1/agents`, `GET /api/v1/agents`, `PUT /api/v1/agents/:id`
- [ ] Admin frontend: AgentManagement, CreateAgentModal, EditAgentModal
- [ ] Channel membership: add/remove agent to channels
- [ ] Basic memory: store/retrieve conversation summaries

### Phase 2: Streaming + RAG
- [ ] `complete_stream` implementation for all providers
- [ ] Progressive message posting (typing indicator → partial text → final)
- [ ] pgvector extension + embedding generation
- [ ] Semantic memory search in context building
- [ ] File attachment support (agent can reference uploaded files in context)

### Phase 3: Advanced Capabilities
- [ ] Tool calling (agent can call internal APIs: search, create reminders, etc.)
- [ ] Multi-agent conversations (agents can ping each other)
- [ ] Agent playbook integration (triggered by runbook steps)
- [ ] Custom provider plugins (webhook-based providers)

---

## Alternatives Considered

### A. Separate `agents` table instead of reusing `users`
**Rejected:** Would require duplicating auth, presence, channel membership, mention parsing, and WebSocket broadcast logic. The existing `users` + `entity_type` pattern is exactly designed for this.

### B. Store agent config in `users.entity_metadata` JSONB
**Rejected:** No schema enforcement, no FK constraints, no indexed queries for admin dashboards. A separate `agent_configs` table with typed columns is cleaner and more maintainable.

### C. Use an external message queue (RabbitMQ, Kafka) for agent triggers
**Rejected:** Overkill for Phase 1. The existing WebSocket event + Tokio task model is sufficient. If volume grows beyond what a single node can handle, we can add a Redis-backed job queue later without changing the architecture.

### D. Build a dedicated microservice for agent runtime
**Rejected:** Adds deployment complexity, network latency, and auth overhead. The agent runtime is a service module inside the monolith. It can be extracted later if needed.

---

## Related Files (Current State)

| File | Relevance |
|------|-----------|
| `backend/src/models/entity.rs` | EntityType enum (Human, Agent, Service, CI) |
| `backend/src/models/user.rs` | User struct with `entity_type`, `api_key_hash`, `entity_metadata` |
| `backend/src/api/v1/entities.rs` | Admin entity registration endpoint |
| `backend/src/auth/extractors.rs` | `ApiKeyAuth`, `PolymorphicAuth` |
| `backend/src/api/v4/ai.rs` | Stub AI endpoints (to be replaced) |
| `backend/src/services/posts.rs` | Mention parsing (`parse_mentions`), post creation |
| `backend/src/realtime/hub.rs` | WebSocket broadcast hub |
| `frontend/src/views/admin/` | Admin console views |
| `frontend/src/features/admin/` | Admin store + API client |
