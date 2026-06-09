-- Agent memory table
-- Stores conversation summaries, facts, and preferences per agent per channel.
-- Phase 1: exact-match retrieval. Phase 2: pgvector semantic search.

CREATE TABLE IF NOT EXISTS agent_memories (
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

CREATE INDEX IF NOT EXISTS idx_agent_memories_agent_channel ON agent_memories(agent_id, channel_id);
CREATE INDEX IF NOT EXISTS idx_agent_memories_created_at ON agent_memories(created_at);
CREATE INDEX IF NOT EXISTS idx_agent_memories_expires_at ON agent_memories(expires_at) WHERE expires_at IS NOT NULL;

-- Prevent duplicate exact memories per agent/channel/type
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_memories_unique_content
    ON agent_memories(agent_id, channel_id, memory_type, content);

-- Auto-update updated_at timestamp
DROP TRIGGER IF EXISTS trg_agent_memories_updated_at ON agent_memories;
CREATE TRIGGER trg_agent_memories_updated_at
    BEFORE UPDATE ON agent_memories
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_configs_updated_at();
