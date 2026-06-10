# ADR-003 Implementation Spec: Agents Phase 3

**Date:** 2026-06-09
**Status:** Draft
**Issue:** [#177](https://github.com/kubedoio/rustchat/issues/177)
**Depends on:** ADR-002 (RAG Knowledge Base — complete)

---

## Goal

Implement streaming agent responses, tool calling, message feedback, and hybrid RAG.

---

## Slice Breakdown

### Slice 1: Streaming Agent Responses via WebSocket

**Objective:** When an agent generates a response, stream tokens to the frontend in real time instead of blocking until completion.

**Backend changes:**

1. **Add `stream` support to `LlmProvider` trait**
   ```rust
   #[async_trait]
   pub trait LlmProvider: Send + Sync {
       async fn generate(&self, messages: &[ChatMessage], config: &LlmConfig) -> ApiResult<String>;
       async fn generate_stream(
           &self,
           messages: &[ChatMessage],
           config: &LlmConfig,
       ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>>;
   }
   ```

2. **Implement `generate_stream` in `OpenAiProvider`**
   - Use `reqwest` with `stream: true` in the request body
   - Parse SSE (Server-Sent Events) format from OpenAI
   - Yield each `choices[0].delta.content` chunk as a `String`

3. **Add WebSocket message types**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum AgentStreamEvent {
       Chunk { content: String, channel_id: Uuid, agent_id: Uuid },
       Complete { post_id: Uuid, channel_id: Uuid, agent_id: Uuid },
       Error { message: String, channel_id: Uuid, agent_id: Uuid },
   }
   ```

4. **Update `AgentRuntime::run_agent_response`**
   - If streaming is enabled (always true for Phase 3), use `generate_stream`
   - Accumulate chunks in a `String`
   - Send `AgentStreamEvent::Chunk` via `ws_hub` every 100ms or every 5 tokens
   - On completion: persist final message, send `AgentStreamEvent::Complete`
   - On error: send `AgentStreamEvent::Error`

5. **Update `WsHub` to broadcast agent stream events**
   - Add `broadcast_agent_stream(&self, channel_id: Uuid, event: AgentStreamEvent)`

**Frontend changes:**

1. **Handle `agent_stream` WebSocket events in message store**
   - Add a `streamingMessages: Map<string, Message>` to the message store
   - On `agent_stream` chunk: create/update a temporary message with `isStreaming: true`
   - On `agent_stream_complete`: replace temporary with real persisted message
   - On `agent_stream_error`: show error state

2. **Update `MessageItem.vue`**
   - Add `isStreaming` prop/visual indicator (pulsing cursor `▋` at end of text)
   - Apply markdown rendering even for streaming partial text

**Verification:**
- Trigger an agent mention → tokens appear in real time
- Refresh page → streamed message is persisted and loaded normally

---

### Slice 2: Tool Calling Framework

**Objective:** Agents can invoke registered tools during response generation.

**Backend:**

1. **Create `backend/src/services/tools/mod.rs`**
   ```rust
   pub mod registry;
   pub mod executor;
   ```

2. **Create `backend/src/services/tools/registry.rs`**
   ```rust
   #[async_trait]
   pub trait Tool: Send + Sync {
       fn name(&self) -> &str;
       fn description(&self) -> &str;
       fn schema(&self) -> serde_json::Value;
       async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError>;
   }

   pub struct ToolRegistry {
       tools: DashMap<String, Arc<dyn Tool>>,
   }

   impl ToolRegistry {
       pub fn new() -> Self { ... }
       pub fn register(&self, tool: Arc<dyn Tool>) { ... }
       pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> { ... }
       pub fn list(&self) -> Vec<Arc<dyn Tool>> { ... }
       pub fn schemas_json(&self) -> serde_json::Value { ... }
   }
   ```

3. **Create `backend/src/services/tools/executor.rs`**
   ```rust
   pub struct ToolExecutor {
       registry: Arc<ToolRegistry>,
       max_iterations: usize,
   }

   impl ToolExecutor {
       /// Runs the LLM with tool context, executes any tool calls, and loops.
       pub async fn execute_with_tools(
           &self,
           provider: &dyn LlmProvider,
           messages: Vec<ChatMessage>,
           config: &LlmConfig,
       ) -> Result<String, ToolExecutionError> { ... }
   }
   ```

4. **Parse tool calls from LLM response**
   - Use XML format: `<tool_call name="...">{ "arg": "value" }</tool_call>`
   - Or JSON format: `{"tool": "...", "arguments": {}}`
   - System prompt instructs LLM to use the XML format

5. **Integrate into `AgentRuntime`**
   - Construct `ToolExecutor` with available tools
   - If agent has tools enabled, use `execute_with_tools` instead of `generate`

**Frontend:** No changes needed for Slice 2.

**Verification:**
- Register a mock echo tool → agent can invoke it
- Verify max iterations limit prevents loops

---

### Slice 3: Web Search Tool

**Objective:** First concrete tool — agents can search the web for live information.

**Backend:**

1. **Create `backend/src/services/tools/web_search.rs`**
   ```rust
   pub struct WebSearchTool {
       client: reqwest::Client,
       api_key: String,
       engine: SearchEngine,
   }

   pub enum SearchEngine {
       Serper,      // serper.dev
       Tavily,      // tavily.com
       DuckDuckGo,  // html scrape fallback
   }

   #[async_trait]
   impl Tool for WebSearchTool {
       fn name(&self) -> &str { "web_search" }
       fn description(&self) -> &str { "Search the web for current information" }
       fn schema(&self) -> serde_json::Value {
           json!({
               "type": "object",
               "properties": {
                   "query": { "type": "string", "description": "Search query" },
                   "num_results": { "type": "integer", "default": 5 }
               },
               "required": ["query"]
           })
       }
       async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
           let query = args["query"].as_str().unwrap_or_default();
           let num = args["num_results"].as_u64().unwrap_or(5) as usize;
           // Call search API, format results as markdown
       }
   }
   ```

2. **Add env config:**
   ```
   RUSTCHAT_SEARCH_ENGINE=serper
   RUSTCHAT_SEARCH_API_KEY=...
   ```

3. **Register tool in `AgentRuntime`** if API key is present.

**Verification:**
- Ask agent "What's the weather in Berlin?" → agent calls web_search tool → returns current info

---

### Slice 4: Message Feedback (Thumbs Up/Down)

**Objective:** Users can rate agent responses.

**Backend:**

1. **Migration:** `20260611000001_agent_message_feedback.sql`
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
   CREATE INDEX idx_agent_feedback_post ON agent_message_feedback(post_id);
   CREATE INDEX idx_agent_feedback_user ON agent_message_feedback(user_id);
   ```

2. **Model + Repository**
   ```rust
   pub struct AgentMessageFeedback { ... }
   pub struct CreateFeedbackRequest { pub feedback_type: String, pub comment: Option<String> }
   ```

3. **API endpoints**
   ```
   POST   /api/v1/posts/:post_id/feedback     → submit feedback
   GET    /api/v1/posts/:post_id/feedback     → get feedback summary
   DELETE /api/v1/posts/:post_id/feedback     → remove own feedback
   GET    /api/v1/agents/:id/feedback         → admin: feedback stats for agent
   ```

**Frontend:**

1. **Update `MessageItem.vue`** for bot messages
   - Show 👍 / 👎 buttons on hover
   - Highlight selected feedback
   - Optional comment textarea on negative feedback

2. **Update `AgentManagement.vue`**
   - Add "Feedback" tab showing thumbs up/down ratio per agent

**Verification:**
- Rate an agent message → stored in DB
- Admin sees aggregated stats

---

### Slice 5: Hybrid RAG (Meilisearch + pgvector)

**Objective:** Combine semantic and full-text search for better retrieval.

**Backend:**

1. **Update `KnowledgeRepository::search_chunks`** to support hybrid mode
   ```rust
   pub async fn search_chunks_hybrid(
       &self,
       query_text: &str,
       query_embedding: &[f32],
       top_k: i32,
       filter: &SearchFilter,
   ) -> Result<Vec<RetrievedChunk>, sqlx::Error>
   ```

2. **Meilisearch query for knowledge documents**
   - Search `knowledge_documents` index (may need to create it)
   - Or search `posts` index and filter by source_type

3. **Reciprocal Rank Fusion**
   ```rust
   fn rrf_fuse(
       semantic_results: Vec<RetrievedChunk>,
       text_results: Vec<RetrievedChunk>,
       k: f32,
   ) -> Vec<RetrievedChunk> {
       // score = Σ(1 / (rank + k))
       // Sort by score descending
   }
   ```

4. **Update `AgentMemoryService`** to use hybrid search when available.

**Verification:**
- Query with exact file name → full-text search finds it even if semantic misses
- Query with concept → semantic search finds it even if keywords don't match

---

### Slice 6: Agent Analytics Dashboard

**Objective:** Admin can see agent usage, costs, and quality metrics.

**Backend:**

1. **Migration:** `20260611000002_agent_analytics.sql`
   ```sql
   CREATE TABLE agent_usage_logs (
       id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
       agent_id UUID NOT NULL REFERENCES agent_configs(user_id),
       channel_id UUID NOT NULL REFERENCES channels(id),
       trigger_type VARCHAR(16) NOT NULL, -- mention, all
       tokens_input INT NOT NULL DEFAULT 0,
       tokens_output INT NOT NULL DEFAULT 0,
       latency_ms INT NOT NULL DEFAULT 0,
       model VARCHAR(64) NOT NULL,
       cost_usd DECIMAL(10, 6),
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
   );
   ```

2. **Log usage in `AgentRuntime`** after each response

3. **Aggregation API**
   ```
   GET /api/v1/agents/:id/analytics?period=7d
   → { total_invocations, total_tokens, total_cost, avg_latency, feedback_ratio }
   ```

**Frontend:**
- Add "Analytics" tab to `AgentManagement.vue`
- Show charts: invocations over time, token usage, cost, feedback ratio

---

### Slice 7: E2E Tests + Rate Limiting + Polish

**Objective:** Production hardening.

**Backend:**

1. **Per-agent rate limiting**
   - Max mentions per minute: 10
   - Max tokens per hour: 100K
   - Store counters in Redis or in-memory sliding window

2. **Error handling improvements**
   - Graceful degradation if LLM API is down (show "Agent is temporarily unavailable")
   - Timeout on tool execution (30s max)
   - Retry with exponential backoff on transient errors

3. **Integration tests**
   - Full pipeline: upload → extract → index → agent mention → RAG context → response
   - Tool execution: mock tool registry, verify loop behavior
   - Streaming: verify WebSocket events arrive in order

**Frontend:**
- E2E test for agent mention with streaming
- Error state UI for agent failures

---

## Dependency Graph

```
Slice 1 (Streaming)
    │
    ▼
Slice 2 (Tool framework)
    │
    ├─▶ Slice 3 (Web search tool)
    │
    ▼
Slice 4 (Feedback) ──▶ Slice 6 (Analytics)
    │
    ▼
Slice 5 (Hybrid RAG)
    │
    ▼
Slice 7 (Tests + polish)
```

**Parallelizable:**
- Slice 1, Slice 4, and Slice 5 can be worked on in parallel after nothing (they're independent).
- Slice 2 depends on nothing but enables Slice 3.
- Slice 6 depends on Slice 4.
- Slice 7 is last.

---

## Open Questions

1. **Which search engine for web search?** Serper ($50/mo) is reliable; Tavily has a generous free tier; DuckDuckGo scraping is brittle but free.
2. **Rate limiting backend:** Do we have Redis? If not, in-memory `DashMap` with TTL is sufficient for single-instance deployments.
3. **Streaming UI:** Should streaming messages be editable by the agent? No — only the final persisted message is editable.
4. **Tool results visibility:** Should tool execution results be visible to users? Suggested: show a collapsed "Thought process" section with tool calls and results.
