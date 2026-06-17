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

## Create an Agent

1. Open **Admin Console > AI Agents**.
2. Create an agent with a display name, username, model, system prompt, temperature, and context limits.
3. Review the generated API key if your deployment uses agent-to-agent or external agent calls.
4. Save the agent.

Agents are stored as normal RustChat users with `entity_type = 'agent'`. They appear in channel membership and post authorship using their configured profile.

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
