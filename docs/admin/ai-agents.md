# AI Agents Administration

AI Agents let administrators create assistant accounts that participate in channels, answer mentions, use configured knowledge bases, and collect feedback on generated responses.

The runtime is optional. If no LLM provider key is configured, RustChat starts normally but logs that the agent runtime is disabled. Configure a provider before expecting agents to generate replies.

## Prerequisites

| Requirement | Purpose |
|-------------|---------|
| `RUSTCHAT_OPENAI_API_KEY` or `OPENAI_API_KEY` | Enables the OpenAI LLM provider and agent runtime |
| PostgreSQL `pgvector` extension | Required for RAG knowledge base vector search |
| S3-compatible storage | Stores uploaded knowledge documents |
| `TAVILY_API_KEY` | Optional web search tool for agents |

Recommended environment:

```bash
RUSTCHAT_OPENAI_API_KEY=sk-...
RUSTCHAT_OPENAI_MODEL=gpt-4o-mini
RUSTCHAT_OPENAI_MAX_TOKENS=2048
```

Agent rate limits (requests per minute and tokens per hour) are internal constants, not configurable via environment variables.

For RAG:

```sql
CREATE EXTENSION IF NOT EXISTS vector;
```

## Create and Configure an Agent

To create a new AI Agent, navigate to **Admin Console > AI Agents** and click **Create Agent**. The configuration is divided into four main tabs:

### 1. Basic Details
* **Username**: The unique handle for the agent (e.g. `support-bot`). The username must be unique, lowercase alphanumeric with dashes/underscores, and is used for `@mentions`.
* **Email**: A valid email address mapped to this agent account.
* **Display Name**: (Optional) The public-facing name shown in chat threads.
* **Title**: A short title indicating the agent's role (e.g., *Customer Support AI*).
* **Description**: (Optional) Admin-facing notes about the agent's purpose.

### 2. LLM Provider Options
* **Provider**: Select from `OpenAI`, `Anthropic`, or `Ollama`.
* **Model**: Specify the target model ID (e.g., `gpt-4o-mini`, `claude-3-5-sonnet-20241022`, or local Ollama tags).
* **API Token**: (Optional) Override credentials for this specific agent. If left blank, the system-wide environment variables (e.g., `RUSTCHAT_OPENAI_API_KEY`) are utilized.
* **Temperature**: Controls response randomness (range `0.0` to `2.0`). Recommended `0.3` for grounded tasks and `0.7` for general conversation.
* **Max Output Tokens**: Limits the response token length (e.g., `1024`).

### 3. Behavior & Capabilities
* **System Prompt**: Core instructions defining the agent's persona, boundaries, response style, and instructions.
* **Max Context Messages**: The maximum number of historical messages in the conversation channel sent as context to the LLM (default `10`).
* **Capabilities**:
  * **Respond to Mentions**: When checked, the agent replies when explicitly `@mentioned` in channels.
  * **Respond to All**: When checked, the agent processes and replies to *every* new message in assigned channels.
  * **Use Memory**: Enables the agent to maintain basic message history/memory.
  * **Use RAG**: Allows the agent to query assigned knowledge bases.
* **RAG Settings**:
  * **RAG Enabled**: (Toggle) Explicitly enable Retrieval-Augmented Generation.
  * **RAG Top-K**: Number of relevant document chunks retrieved during semantic vector search queries (default `5`).

### 4. Channels
* Select which channels the agent is active in. Agents can only participate in, read, or reply to channels they are explicitly added to.

---

Agents are stored as normal RustChat users with `entity_type = 'agent'`. They appear in channel membership lists and post authorship using their configured profiles. If your deployment uses external agent calls or agent-to-agent (a2a) communication, you can retrieve the agent's generated API key from the agent detail view.

## Assign Agents to Channels

1. Open the agent detail view.
2. Add one or more channels.
3. Choose whether the agent responds only to mentions or responds to all channel messages.
4. Save the assignment.

Agents cannot read channels they are not assigned to. Removing a channel assignment stops future responses in that channel.

## Knowledge Bases

Knowledge bases give agents grounded context from uploaded or synced documents.

1. Open **Admin Console > Knowledge Bases**.
2. Create a knowledge base for a team or domain.
3. Upload documents or configure a sync source.
4. Wait for indexing to complete.
5. Assign the knowledge base to one or more agents.

Document blobs are stored in S3-compatible storage. Metadata, chunks, and embeddings are stored in PostgreSQL. If a newly uploaded document is not indexed yet, agents may answer without that document until ingestion finishes.

## RustShare Sync Sources

RustShare sync sources map external folders into a RustChat knowledge base. Use a dedicated integration credential with the minimum scope required for the folder being synced.

Operational notes:
- Prefer one knowledge base per product, team, or policy domain.
- Keep sensitive folders separate so assignments can be reviewed per agent.
- Monitor indexing errors after large syncs.
- Revoke or rotate sync credentials when ownership changes.

## Tools

Tools are server-side capabilities that agents may call during response generation. They are disabled unless configured by the operator.

The web search tool is registered when `TAVILY_API_KEY` is set. Remove the key and restart the backend to disable it.

## Feedback and Analytics

Users can submit positive or negative feedback on agent posts. Admins can review aggregate feedback and usage analytics per agent to identify poor prompts, noisy channels, or cost spikes.

Review:
- response volume by agent and channel
- positive and negative feedback rates
- latency and token usage
- channels where `respond_to_all` causes excessive responses

## Troubleshooting

### Agent Does Not Respond

Check:
- An LLM provider key is configured and the backend was restarted.
- The agent is active.
- The agent is assigned to the channel.
- The message mentions the agent username, unless `respond_to_all` is enabled.
- Rate limits have not been exceeded.
- Backend logs do not show LLM provider errors.

### Knowledge Answers Are Missing Context

Check:
- PostgreSQL has the `pgvector` extension.
- The document is uploaded or synced into the expected knowledge base.
- The document has finished indexing.
- The knowledge base is assigned to the agent.
- The embedding provider key is configured.

### Web Search Is Unavailable

Check:
- `TAVILY_API_KEY` is configured.
- The backend was restarted after the key was added.
- Outbound network access is allowed from the backend.

## Security Notes

- Treat provider API keys and sync credentials as production secrets.
- Keep agent prompts free of secrets; prompts may be sent to external LLM providers.
- Assign agents only to channels they need.
- Review knowledge base contents before assigning them to broadly visible agents.
- Disable optional tools when they are not needed.
