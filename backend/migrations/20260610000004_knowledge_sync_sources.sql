-- Knowledge sync sources table
-- Stores external source configurations (e.g., RustShare) for automatic document syncing.

CREATE TABLE IF NOT EXISTS knowledge_sync_sources (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,

    name VARCHAR(256) NOT NULL,
    source_type VARCHAR(64) NOT NULL,
    config_encrypted TEXT NOT NULL,
    sync_mode VARCHAR(32) NOT NULL,
    sync_interval_minutes INT,

    is_active BOOLEAN NOT NULL DEFAULT TRUE,

    last_sync_at TIMESTAMPTZ,
    last_sync_status VARCHAR(32),
    last_sync_error TEXT,
    next_sync_at TIMESTAMPTZ,
    document_count INT NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_knowledge_sync_sources_team_id ON knowledge_sync_sources(team_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_sync_sources_is_active ON knowledge_sync_sources(is_active);

-- Add sync_source_id FK to knowledge_documents (must happen after both tables exist)
ALTER TABLE knowledge_documents
    ADD COLUMN IF NOT EXISTS sync_source_id UUID REFERENCES knowledge_sync_sources(id);
CREATE INDEX IF NOT EXISTS idx_knowledge_documents_source ON knowledge_documents(sync_source_id);

-- Auto-update updated_at timestamp
DROP TRIGGER IF EXISTS trg_knowledge_sync_sources_updated_at ON knowledge_sync_sources;
CREATE TRIGGER trg_knowledge_sync_sources_updated_at
    BEFORE UPDATE ON knowledge_sync_sources
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_configs_updated_at();
