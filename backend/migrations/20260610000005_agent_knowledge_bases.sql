-- Agent knowledge base junction table
-- Maps agents to knowledge bases with per-assignment RAG configuration.

CREATE TABLE IF NOT EXISTS agent_knowledge_bases (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent_configs(user_id) ON DELETE CASCADE,
    knowledge_base_id UUID NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,

    top_k INT NOT NULL DEFAULT 5 CHECK (top_k BETWEEN 1 AND 20),
    relevance_threshold REAL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(agent_id, knowledge_base_id)
);

CREATE INDEX IF NOT EXISTS idx_agent_knowledge_bases_agent_id ON agent_knowledge_bases(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_knowledge_bases_kb_id ON agent_knowledge_bases(knowledge_base_id);

-- Auto-update updated_at timestamp
DROP TRIGGER IF EXISTS trg_agent_knowledge_bases_updated_at ON agent_knowledge_bases;
CREATE TRIGGER trg_agent_knowledge_bases_updated_at
    BEFORE UPDATE ON agent_knowledge_bases
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_configs_updated_at();
