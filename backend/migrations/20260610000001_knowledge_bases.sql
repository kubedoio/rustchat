-- Knowledge bases table
-- Stores configuration for RAG knowledge bases, including embedding and chunking settings.

CREATE TABLE IF NOT EXISTS knowledge_bases (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,

    name VARCHAR(256) NOT NULL,
    description TEXT,

    embedding_model VARCHAR(64) NOT NULL,
    embedding_dimensions INT NOT NULL,

    chunk_size INT NOT NULL DEFAULT 512,
    chunk_overlap INT NOT NULL DEFAULT 50,

    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_bases_team_id ON knowledge_bases(team_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_bases_is_active ON knowledge_bases(is_active);

-- Auto-update updated_at timestamp
DROP TRIGGER IF EXISTS trg_knowledge_bases_updated_at ON knowledge_bases;
CREATE TRIGGER trg_knowledge_bases_updated_at
    BEFORE UPDATE ON knowledge_bases
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_configs_updated_at();
