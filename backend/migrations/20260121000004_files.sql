-- Files table for rustchat
-- Migration: 004_files

CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    uploader_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id UUID REFERENCES channels(id) ON DELETE SET NULL,
    post_id UUID REFERENCES posts(id) ON DELETE SET NULL,
    
    -- File info
    name VARCHAR(255) NOT NULL,
    key VARCHAR(512) NOT NULL UNIQUE,
    mime_type VARCHAR(128) NOT NULL,
    size BIGINT NOT NULL,
    
    -- Storage backend
    backend VARCHAR(32) NOT NULL DEFAULT 's3',
    
    -- Image metadata
    width INTEGER,
    height INTEGER,
    has_thumbnail BOOLEAN NOT NULL DEFAULT FALSE,
    thumbnail_key VARCHAR(512),
    
    -- Checksums
    sha256 VARCHAR(64),
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_files_uploader_id ON files(uploader_id);
CREATE INDEX idx_files_channel_id ON files(channel_id);
CREATE INDEX idx_files_post_id ON files(post_id);
CREATE INDEX idx_files_key ON files(key);

-- Full-text search index for posts
CREATE INDEX idx_posts_message_search ON posts USING GIN(to_tsvector('english', message));
