-- Agent channel settings table
-- Extends channel_members with agent-specific overrides per channel.
-- Allows per-channel prompt overrides and activation toggles.

CREATE TABLE IF NOT EXISTS agent_channel_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent_configs(user_id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,

    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    custom_prompt_override TEXT,
    max_context_messages_override INT CHECK (max_context_messages_override BETWEEN 1 AND 100),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(agent_id, channel_id)
);

CREATE INDEX IF NOT EXISTS idx_agent_channel_settings_agent ON agent_channel_settings(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_channel_settings_channel ON agent_channel_settings(channel_id);

-- Auto-update updated_at timestamp
DROP TRIGGER IF EXISTS trg_agent_channel_settings_updated_at ON agent_channel_settings;
CREATE TRIGGER trg_agent_channel_settings_updated_at
    BEFORE UPDATE ON agent_channel_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_configs_updated_at();
