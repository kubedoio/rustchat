-- Knowledge documents table
-- Stores metadata for uploaded documents and their extracted text.

CREATE TABLE IF NOT EXISTS knowledge_documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    knowledge_base_id UUID NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,

    title VARCHAR(512) NOT NULL,
    source_url TEXT,
    source_type VARCHAR(32) NOT NULL DEFAULT 'upload',
    s3_key TEXT NOT NULL,
    s3_bucket TEXT NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    mime_type VARCHAR(128) NOT NULL,
    size_bytes BIGINT NOT NULL DEFAULT 0,

    extracted_text TEXT,
    extracted_at TIMESTAMPTZ,

    external_id TEXT,
    external_etag TEXT,
    external_modified_at TIMESTAMPTZ,

    is_indexed BOOLEAN NOT NULL DEFAULT FALSE,
    chunk_count INT NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_documents_kb ON knowledge_documents(knowledge_base_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_documents_team ON knowledge_documents(team_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_documents_hash ON knowledge_documents(content_hash);


-- Auto-update updated_at timestamp
DROP TRIGGER IF EXISTS trg_knowledge_documents_updated_at ON knowledge_documents;
CREATE TRIGGER trg_knowledge_documents_updated_at
    BEFORE UPDATE ON knowledge_documents
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_configs_updated_at();
