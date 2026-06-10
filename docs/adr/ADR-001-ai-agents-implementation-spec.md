# Spec: AI Agents Implementation

**ADR:** [ADR-001: AI Agents as Channel Participants](./ADR-001-ai-agents-architecture.md)
**Issue:** [#177](https://github.com/kubedoio/rustchat/issues/177)
**Date:** 2026-06-09

---

## Objective

Implement AI agents as first-class channel participants in RustChat. An admin can create, configure, and assign agents to channels. When a user mentions an agent (`@agent-name`), the agent reads channel context, calls an LLM, and posts a reply. Agents have memory, configurable prompts, and channel-specific permissions.

## Success Criteria

- [ ] Admin can create an agent via UI with: name, username, avatar, LLM provider, model, API token, system prompt, temperature, max context messages
- [ ] Admin can assign/unassign agents to channels
- [ ] Agent appears in channel member list
- [ ] When `@agent-username` is mentioned in a channel, the agent responds within 5 seconds (OpenAI GPT-4o-mini baseline)
- [ ] Agent response includes recent channel context (last N messages)
- [ ] Agent memory persists across conversations (stored in DB)
- [ ] Agent cannot read channels it is not assigned to
- [ ] All new code has unit tests; agent runtime has integration tests
- [ ] No regression in existing CI (backend check, frontend check, E2E)

## Tech Stack

- **Backend:** Rust (Axum, sqlx, tokio), PostgreSQL, Redis
- **Frontend:** Vue 3, TypeScript, Pinia, Tailwind CSS v4
- **LLM:** OpenAI API (Phase 1), extensible trait for Anthropic/Ollama
- **Encryption:** AES-256-GCM (existing `backend/src/auth/crypto.rs` or `aes-gcm` crate)

---

## Project Structure (New Files)

```
backend/
├── migrations/
│   ├── 20260609000001_agent_configs.sql
│   ├── 20260609000002_agent_memories.sql
│   └── 20260609000003_agent_channel_memberships.sql
├── src/
│   ├── models/
│   │   └── agent.rs              # AgentConfig, AgentMemory, AgentCapability
│   ├── repositories/
│   │   └── agent_repository.rs   # CRUD for agent_configs, agent_memories
│   ├── services/
│   │   ├── llm/
│   │   │   ├── mod.rs            # LlmProvider trait, CompletionRequest, ChatMessage
│   │   │   ├── openai.rs         # OpenAiProvider
│   │   │   ├── anthropic.rs      # AnthropicProvider (stub for Phase 1)
│   │   │   ├── ollama.rs         # OllamaProvider (stub for Phase 1)
│   │   │   └── registry.rs       # ProviderRegistry
│   │   ├── agent_runtime.rs      # AgentRuntime service + trigger pipeline
│   │   └── agent_memory.rs       # Memory builder / retriever
│   ├── api/
│   │   ├── v1/
│   │   │   └── agents.rs         # REST endpoints
│   │   └── mod.rs                # Router nesting (update)
│   └── auth/
│       └── crypto.rs             # Token encryption/decryption helpers
frontend/
├── src/
│   ├── api/
│   │   └── agents.ts             # Agent API client
│   ├── features/
│   │   └── admin/
│   │       ├── stores/
│   │       │   └── agentStore.ts # Pinia store for agents
│   │       └── services/
│   │           └── agentService.ts # Business logic
│   ├── components/
│   │   ├── admin/
│   │   │   ├── AgentForm.vue     # Shared create/edit form
│   │   │   ├── AgentTable.vue    # Admin list table
│   │   │   └── AgentChannelPicker.vue
│   │   └── modals/
│   │       ├── CreateAgentModal.vue
│   │       └── EditAgentModal.vue
│   └── views/
│       └── admin/
│           └── AgentManagement.vue
```

---

## Code Style

### Backend

- Use `sqlx::query_as!` for compile-time checked queries.
- All DB operations go through `AgentRepository` (no raw SQL in handlers).
- `AgentRuntime` methods are `async` and return `Result<T, AppError>`.
- LLM provider errors map to `AppError::ExternalService` with structured logging.
- Use `tracing::instrument` on all service methods.

Example:
```rust
#[tracing::instrument(skip(self, request), fields(agent_id = %agent_id))]
pub async fn handle_mention(&self, agent_id: Uuid, request: AgentTriggerRequest) -> Result<(), AppError> {
    let config = self.agent_repo.get_config(agent_id).await?;
    let context = self.build_context(&config, request.channel_id).await?;
    let response = self.llm.complete(config.provider_request(context)).await?;
    self.post_service.create_as_agent(agent_id, request.channel_id, response).await?;
    self.memory_service.store_turn(agent_id, request.channel_id, &request, &response).await?;
    Ok(())
}
```

### Frontend

- Components use `<script setup lang="ts">`.
- Props are typed with TypeScript interfaces.
- API calls go through `agentsApi` module, not inline fetch.
- Store mutations are synchronous; async actions use `async/await`.

Example:
```vue
<script setup lang="ts">
import { computed } from 'vue';
import { useAgentStore } from '@/features/admin/stores/agentStore';

const agentStore = useAgentStore();
const agents = computed(() => agentStore.agents);
</script>
```

---

## Testing Strategy

### Backend

| Layer | Framework | Location | Coverage Target |
|-------|-----------|----------|-----------------|
| LLM Provider | Unit (mock HTTP server) | `backend/src/services/llm/` | 80% |
| Agent Repository | Integration (sqlx test transactions) | `backend/src/repositories/` | 80% |
| Agent Runtime | Integration (mock LLM + in-memory DB) | `backend/tests/agent_runtime/` | 70% |
| API Endpoints | Integration (axum TestServer) | `backend/tests/api_agents.rs` | 70% |
| Encryption | Unit | `backend/src/auth/crypto.rs` | 90% |

**Mock LLM Provider for tests:**
```rust
struct MockProvider {
    response: String,
    delay_ms: u64,
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        Ok(CompletionResponse { content: self.response.clone(), usage: None })
    }
}
```

### Frontend

| Layer | Framework | Location |
|-------|-----------|----------|
| Agent Store | Vitest unit | `frontend/src/features/admin/stores/agentStore.test.ts` |
| Agent Form | Vitest component | `frontend/src/components/admin/AgentForm.test.ts` |
| Agent API | Vitest (msw mock) | `frontend/src/api/agents.test.ts` |

---

## Boundaries

### Always
- Run `cargo test` and `cargo clippy` before commits.
- Add tracing spans to all new service methods.
- Encrypt LLM API tokens before DB insertion.
- Validate all user inputs at API boundary (max lengths, ranges).
- Use existing error types (`AppError`) — no new error enums for HTTP layer.

### Ask First
- Adding new crates to `Cargo.toml` (security review needed).
- Changing `users` table schema (affects all auth).
- Modifying WebSocket event format (affects MM compat).
- Enabling `respond_to_all` by default (product decision).

### Never
- Store LLM API tokens in plaintext.
- Allow agents to bypass channel membership checks.
- Commit `.env` files with real API keys.
- Block HTTP/WebSocket handlers waiting for LLM response.
- Skip migration files — every schema change must be reversible.

---

## Migration Files

### `20260609000001_agent_configs.sql`
```sql
-- Enable pgcrypto if not already enabled (for encryption helpers)
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Agent configuration table
CREATE TABLE agent_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(128) NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL DEFAULT '',
    provider VARCHAR(32) NOT NULL DEFAULT 'openai',
    model VARCHAR(64) NOT NULL DEFAULT 'gpt-4o-mini',
    api_token_encrypted TEXT,
    temperature REAL NOT NULL DEFAULT 0.7 CHECK (temperature BETWEEN 0.0 AND 2.0),
    max_context_messages INT NOT NULL DEFAULT 20 CHECK (max_context_messages BETWEEN 1 AND 100),
    max_output_tokens INT NOT NULL DEFAULT 1024 CHECK (max_output_tokens BETWEEN 1 AND 8192),
    capabilities JSONB NOT NULL DEFAULT '{"respond_to_mentions": true, "respond_to_all": false, "use_memory": true, "use_rag": false}',
    rag_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    rag_top_k INT NOT NULL DEFAULT 5 CHECK (rag_top_k BETWEEN 1 AND 20),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id)
);

CREATE INDEX idx_agent_configs_provider ON agent_configs(provider);
CREATE INDEX idx_agent_configs_is_active ON agent_configs(is_active);

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_agent_configs_updated_at
    BEFORE UPDATE ON agent_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

### `20260609000002_agent_memories.sql`
```sql
-- Agent memory table (Phase 1: exact-match retrieval; Phase 2: pgvector)
CREATE TABLE agent_memories (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent_configs(user_id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    memory_type VARCHAR(32) NOT NULL DEFAULT 'conversation',
    content TEXT NOT NULL,
    message_ids UUID[],
    importance_score REAL NOT NULL DEFAULT 1.0 CHECK (importance_score BETWEEN 0.0 AND 1.0),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agent_memories_agent_channel ON agent_memories(agent_id, channel_id);
CREATE INDEX idx_agent_memories_created_at ON agent_memories(created_at);
CREATE INDEX idx_agent_memories_expires_at ON agent_memories(expires_at) WHERE expires_at IS NOT NULL;

CREATE TRIGGER update_agent_memories_updated_at
    BEFORE UPDATE ON agent_memories
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

### `20260609000003_agent_channel_memberships.sql`
```sql
-- Explicit agent channel membership tracking (extends channel_members)
-- This table allows agent-specific channel settings without polluting channel_members
CREATE TABLE agent_channel_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent_configs(user_id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    custom_prompt_override TEXT,
    max_context_messages_override INT CHECK (max_context_messages_override BETWEEN 1 AND 100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(agent_id, channel_id)
);

CREATE INDEX idx_agent_channel_settings_agent ON agent_channel_settings(agent_id);
CREATE INDEX idx_agent_channel_settings_channel ON agent_channel_settings(channel_id);

CREATE TRIGGER update_agent_channel_settings_updated_at
    BEFORE UPDATE ON agent_channel_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

---

## API Contract

### `GET /api/v1/agents`
**Auth:** Admin only (`system_admin` or `org_admin`)
**Response:**
```json
{
  "agents": [
    {
      "id": "uuid",
      "user_id": "uuid",
      "username": "code-assistant",
      "display_name": "Code Assistant",
      "avatar_url": "https://...",
      "title": "Code Review Bot",
      "provider": "openai",
      "model": "gpt-4o-mini",
      "is_active": true,
      "channel_count": 3,
      "created_at": "2026-06-09T12:00:00Z"
    }
  ]
}
```

### `POST /api/v1/agents`
**Auth:** Admin only
**Request:**
```json
{
  "username": "code-assistant",
  "email": "code-assistant@rustchat.local",
  "display_name": "Code Assistant",
  "title": "Code Review Bot",
  "description": "Helps with code reviews",
  "system_prompt": "You are a senior software engineer...",
  "provider": "openai",
  "model": "gpt-4o-mini",
  "api_token": "sk-...",
  "temperature": 0.7,
  "max_context_messages": 20,
  "max_output_tokens": 1024,
  "capabilities": {
    "respond_to_mentions": true,
    "respond_to_all": false,
    "use_memory": true,
    "use_rag": false
  },
  "channel_ids": ["uuid-1", "uuid-2"]
}
```
**Response:** `201 Created` with agent object + generated API key.

### `GET /api/v1/agents/:id`
**Auth:** Admin or agent creator
**Response:** Full config (excluding `api_token_encrypted`).

### `PUT /api/v1/agents/:id`
**Auth:** Admin or agent creator
**Request:** Same as POST, but partial (PATCH semantics).
**Response:** Updated agent object.

### `DELETE /api/v1/agents/:id`
**Auth:** Admin only
**Behavior:** Soft delete user row (`deleted_at = NOW()`), cascades to agent_configs.
**Response:** `204 No Content`.

### `POST /api/v1/agents/:id/channels/:channel_id`
**Auth:** Admin or agent creator
**Behavior:** Inserts into `channel_members` + `agent_channel_settings`.
**Response:** `201 Created`.

### `DELETE /api/v1/agents/:id/channels/:channel_id`
**Auth:** Admin or agent creator
**Behavior:** Deletes from `channel_members` + `agent_channel_settings`.
**Response:** `204 No Content`.

---

## Implementation Tasks

### Task 1: Database Layer
- [ ] Write migration files (3 files above)
- [ ] Create `backend/src/models/agent.rs` with structs
- [ ] Create `backend/src/repositories/agent_repository.rs` with CRUD
- [ ] Run `sqlx migrate` and verify schema
- **Acceptance:** `cargo sqlx migrate run` succeeds; `sqlx prepare` generates `.sqlx` files.
- **Verify:** `psql -c "\dt"` shows new tables; FK constraints exist.
- **Files:** `migrations/20260609*`, `src/models/agent.rs`, `src/repositories/agent_repository.rs`

### Task 2: Encryption Helpers
- [ ] Create `backend/src/auth/crypto.rs` with `encrypt_token` / `decrypt_token`
- [ ] Use AES-256-GCM with `ENCRYPTION_KEY` from config (crate `aes-gcm` already in Cargo.toml)
- [ ] Unit tests for round-trip encryption
- **Acceptance:** Token encrypts and decrypts correctly; wrong key fails.
- **Verify:** `cargo test crypto` passes.
- **Files:** `src/auth/crypto.rs`

### Task 3: LLM Provider Abstraction
- [ ] Create `backend/src/services/llm/mod.rs` with trait + types
- [ ] Implement `OpenAiProvider` with `reqwest` HTTP client
- [ ] Create `ProviderRegistry` with env-var initialization
- [ ] Unit tests with mock HTTP server (`wiremock` or `mockito`)
- **Acceptance:** `OpenAiProvider::complete` returns text for a valid request; returns error for 429/401.
- **Verify:** `cargo test llm` passes.
- **Files:** `src/services/llm/*`

### Task 4: Agent Runtime Service
- [ ] Create `backend/src/services/agent_runtime.rs`
- [ ] Implement mention detection hook in `PostService::create_post`
- [ ] Implement context builder (fetch last N messages)
- [ ] Implement response creation (call LLM → create post → broadcast WS)
- [ ] Add per-agent semaphore for concurrency control
- [ ] Integration tests with mock LLM + test DB
- **Acceptance:** Mentioning `@agent-username` in a channel creates a response post within 5s in tests.
- **Verify:** `cargo test agent_runtime` passes.
- **Files:** `src/services/agent_runtime.rs`, `tests/agent_runtime/`

### Task 5: Agent Memory Service
- [ ] Create `backend/src/services/agent_memory.rs`
- [ ] Implement `store_turn` (upsert conversation summary)
- [ ] Implement `retrieve_context` (fetch memories for channel)
- [ ] Implement TTL cleanup (delete expired memories)
- **Acceptance:** Memory persists across requests; expired memories are cleaned.
- **Verify:** `cargo test agent_memory` passes.
- **Files:** `src/services/agent_memory.rs`

### Task 6: REST API Endpoints
- [ ] Create `backend/src/api/v1/agents.rs`
- [ ] Implement all CRUD endpoints
- [ ] Implement channel assignment endpoints
- [ ] Wire into `backend/src/api/mod.rs`
- [ ] Integration tests with axum TestServer
- **Acceptance:** All endpoints return correct status codes; auth enforcement works.
- **Verify:** `cargo test api_agents` passes.
- **Files:** `src/api/v1/agents.rs`, `tests/api_agents.rs`

### Task 7: Frontend API Client
- [ ] Create `frontend/src/api/agents.ts`
- [ ] Implement typed methods for all endpoints
- [ ] Unit tests with `msw`
- **Acceptance:** All API methods compile and return correct types.
- **Verify:** `npm test agents.test.ts` passes.
- **Files:** `frontend/src/api/agents.ts`

### Task 8: Frontend Admin Store
- [ ] Create `frontend/src/features/admin/stores/agentStore.ts`
- [ ] Implement list, create, update, delete actions
- [ ] Unit tests
- **Acceptance:** Store manages agent list correctly; mutations update UI.
- **Verify:** `npm test agentStore.test.ts` passes.
- **Files:** `frontend/src/features/admin/stores/agentStore.ts`

### Task 9: Frontend Admin UI
- [ ] Create `AgentManagement.vue` view
- [ ] Create `AgentForm.vue` component (shared create/edit)
- [ ] Create `AgentTable.vue` component
- [ ] Create `AgentChannelPicker.vue` component
- [ ] Create `CreateAgentModal.vue` and `EditAgentModal.vue`
- [ ] Wire into admin router
- **Acceptance:** Admin can create, edit, delete, and assign agents to channels.
- **Verify:** Manual test + E2E if possible.
- **Files:** `frontend/src/views/admin/AgentManagement.vue`, `frontend/src/components/admin/*`, `frontend/src/components/modals/*`

### Task 10: Frontend Channel UI Updates
- [ ] Update `ChannelSidebar.vue` to show agent members
- [ ] Update `MessageItem.vue` to render agent avatar + badge
- [ ] Update `TypingIndicator.vue` to show agent typing status
- **Acceptance:** Agents visible in sidebar; agent messages have bot badge.
- **Verify:** Visual inspection.
- **Files:** `frontend/src/components/layout/ChannelSidebar.vue`, `frontend/src/components/channel/MessageItem.vue`

### Task 11: CI / Regression
- [ ] Run full backend test suite
- [ ] Run full frontend test suite
- [ ] Run E2E tests
- [ ] Update `.env.example` with new env vars (`RUSTCHAT_OPENAI_API_KEY`, etc.)
- **Acceptance:** Zero regressions; all CI green.
- **Verify:** `cargo test`, `npm test`, `npm run e2e`.
- **Files:** `.env.example`

---

## Open Questions

1. **Should agents have custom avatars or use a default bot icon?**
   - Proposal: Allow upload via existing file system; default to a bot icon from Lucide (`Bot` icon).

2. **Should agent responses support markdown / rich text?**
   - Proposal: Yes, agent responses go through the same markdown pipeline as human messages.

3. **Should we support streaming responses in Phase 1?**
   - Proposal: No. Phase 1 returns full response. Phase 2 adds streaming + progressive display.

4. **What is the default max context window cost budget?**
   - Proposal: 20 messages × ~500 tokens = ~10k input tokens. Admins can adjust per agent.

5. **Should agents be able to initiate conversations (not just respond)?**
   - Proposal: Not in Phase 1. Only mention-triggered responses. Phase 3 adds proactive triggers.
