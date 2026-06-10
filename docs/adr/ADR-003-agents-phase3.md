# ADR-003: Agents Phase 3 — Streaming, Tools, and Feedback

**Date:** 2026-06-09
**Status:** Proposed
**Risk tier:** architectural
**Issue:** [#177](https://github.com/kubedoio/rustchat/issues/177)
**Depends on:** ADR-002 (RAG Knowledge Base)

## Context

Phases 1 and 2 gave RustChat agents identity, memory, RAG grounding, and RustShare sync. Agents can now respond to mentions with contextual, knowledge-augmented answers. However, the experience has three major gaps:

1. **No streaming** — Users wait 5–30 seconds for the full response to appear. This feels broken compared to ChatGPT, Claude, and every other modern AI interface.
2. **No tool use** — Agents cannot search the web, run calculations, query APIs, or interact with external systems. They are limited to pre-synced knowledge and LLM training data.
3. **No feedback loop** — There is no way for users to indicate whether an agent response was helpful. Without this signal, there is no path to improving response quality over time.
4. **RAG is vector-only** — Meilisearch already powers full-text search in RustChat, but RAG retrieval only uses pgvector semantic search. Hybrid search (semantic + keyword) would improve recall for exact-match queries (e.g., file names, error codes, config keys).

## Decision

We will build four subsystems in Phase 3:

### 1. Streaming Responses (WebSocket)

When an agent is triggered, instead of waiting for the full LLM response and then posting it as a single message, the backend streams tokens through the existing WebSocket hub. The frontend receives partial content and renders it in real time, appending tokens as they arrive.

**Backend flow:**
```
Agent triggered
    │
    ▼
LLM provider starts streaming (SSE / chunked JSON)
    │
    ▼
For each token chunk:
    ├─ Append to accumulating message buffer
    ├─ Send WebSocket event: { type: "agent_stream", content: "...", channel_id, agent_id }
    └─ Debounce: every 100ms or every 5 tokens, broadcast to channel subscribers
    │
    ▼
Stream complete
    ├─ Persist final message to posts table
    ├─ Send WebSocket event: { type: "agent_stream_complete", post_id }
    └─ Release semaphore
```

**Frontend flow:**
- Receives `agent_stream` → renders a "typing" indicator message with partial content
- Receives `agent_stream_complete` → transitions to normal persisted message
- If user navigates away and back, the persisted message is loaded from the API as usual

**OpenAI streaming:** Uses `stream: true` with `eventsource` / `reqwest` streaming response parsing.

### 2. Tool Calling Framework

Agents can invoke registered tools during response generation. The LLM receives a system prompt with available tool schemas, can emit `<tool_call>` JSON blocks, and the runtime executes them, feeding results back into the context.

**Tool trait:**
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value; // JSON Schema for parameters
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError>;
}
```

**Tool registry:**
```rust
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn Tool>>,
}
```

**Execution loop:**
```
1. Build context with tool schemas in system prompt
2. Call LLM
3. Parse response for <tool_call> blocks
4. If tool calls found:
   a. Execute each tool in parallel
   b. Append tool results to messages as "function" role
   c. Re-call LLM with updated context
5. Repeat until no more tool calls or max iterations reached
6. Return final response
```

**Max iterations:** 5 (prevents infinite loops).

### 3. Message Feedback (Thumbs Up/Down)

Users can react to agent messages with 👍 / 👎. Feedback is stored per-message with an optional text comment.

**Schema:**
```sql
CREATE TABLE agent_message_feedback (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    feedback_type VARCHAR(8) NOT NULL CHECK (feedback_type IN ('positive', 'negative')),
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id, user_id)
);
```

**Purpose:**
- Surface low-quality responses to admins
- Future: fine-tuning dataset construction, prompt A/B testing, reward model training

### 4. Hybrid RAG (Meilisearch + pgvector)

Combine Meilisearch full-text search with pgvector semantic search for better retrieval.

**Query flow:**
```
User query
    │
    ├─► Embed query → pgvector ANN search (top 10)
    │
    ├─► Query text → Meilisearch full-text search (top 10)
    │
    ▼
Reciprocal Rank Fusion (RRF)
    k=60
    score = Σ(1 / (rank + k))
    │
    ▼
Top 5 fused results
```

**Why RRF:** Simple, parameter-free, empirically strong for combining heterogeneous rankings.

## Consequences

### Positive
- **Streaming** eliminates perceived latency; matches user expectations from ChatGPT/Claude.
- **Tool calling** extends agent capabilities beyond static knowledge to live data and actions.
- **Feedback loop** creates a quality signal for continuous improvement.
- **Hybrid RAG** improves recall for exact-match queries where semantic search alone fails.

### Negative / Risks
- **WebSocket complexity** — Streaming requires careful handling of disconnections, reconnections, and race conditions with normal message loading.
- **Tool safety** — Executing arbitrary tools is a trust boundary. Tools must be sandboxed, read-only by default, and logged.
- **Rate limiting** — Streaming + tool calling increases API cost and load. Per-agent rate limits are needed.
- **RRF overhead** — Two searches per query doubles latency. Mitigation: parallelize searches, cache embeddings.

## Related
- ADR-001: AI Agents as Channel Participants
- ADR-002: RAG Knowledge Base with External Sync
- `docs/adr/ADR-003-agents-phase3-implementation-spec.md` — Phase 3 slice breakdown
