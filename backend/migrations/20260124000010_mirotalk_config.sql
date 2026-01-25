-- MiroTalk Configuration Table
-- Singleton table to store MiroTalk integration settings

CREATE TABLE IF NOT EXISTS mirotalk_config (
    -- Singleton enforcement
    is_active BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (is_active),

    -- Configuration fields
    mode VARCHAR(50) NOT NULL DEFAULT 'disabled', -- 'disabled', 'sfu', 'p2p'
    base_url TEXT NOT NULL DEFAULT '',
    api_key_secret TEXT NOT NULL DEFAULT '',
    default_room_prefix VARCHAR(100),
    join_behavior VARCHAR(50) NOT NULL DEFAULT 'new_tab', -- 'embed_iframe', 'new_tab'

    -- Metadata
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by UUID REFERENCES users(id)
);

-- Insert default row
INSERT INTO mirotalk_config (is_active) VALUES (TRUE) ON CONFLICT (is_active) DO NOTHING;
