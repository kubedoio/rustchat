-- Agent configuration table
-- Stores LLM provider settings, prompts, and behavior configuration for AI agents.
-- Each agent is a users row with entity_type = 'agent'.

CREATE TABLE IF NOT EXISTS agent_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,

    -- Identity & behavior
    title VARCHAR(128) NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL DEFAULT '',

    -- LLM provider settings
    provider VARCHAR(32) NOT NULL DEFAULT 'openai',
    model VARCHAR(64) NOT NULL DEFAULT 'gpt-4o-mini',
    api_token_encrypted TEXT,
    temperature REAL NOT NULL DEFAULT 0.7 CHECK (temperature BETWEEN 0.0 AND 2.0),
    max_context_messages INT NOT NULL DEFAULT 20 CHECK (max_context_messages BETWEEN 1 AND 100),
    max_output_tokens INT NOT NULL DEFAULT 1024 CHECK (max_output_tokens BETWEEN 1 AND 8192),

    -- Capabilities bitmask as JSON for extensibility
    capabilities JSONB NOT NULL DEFAULT '{"respond_to_mentions": true, "respond_to_all": false, "use_memory": true, "use_rag": false}',

    -- RAG settings (Phase 2)
    rag_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    rag_top_k INT NOT NULL DEFAULT 5 CHECK (rag_top_k BETWEEN 1 AND 20),

    -- State
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_agent_configs_provider ON agent_configs(provider);
CREATE INDEX IF NOT EXISTS idx_agent_configs_is_active ON agent_configs(is_active);

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_agent_configs_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

DROP TRIGGER IF EXISTS trg_agent_configs_updated_at ON agent_configs;
CREATE TRIGGER trg_agent_configs_updated_at
    BEFORE UPDATE ON agent_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_configs_updated_at();
