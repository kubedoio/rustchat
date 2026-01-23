-- Integrations tables for rustchat
-- Migration: 005_integrations

-- Incoming webhooks (post to channel)
CREATE TABLE incoming_webhooks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name VARCHAR(255),
    description TEXT,
    token VARCHAR(64) NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Outgoing webhooks (trigger on keywords)
CREATE TABLE outgoing_webhooks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    channel_id UUID REFERENCES channels(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name VARCHAR(255),
    description TEXT,
    trigger_words TEXT[] DEFAULT '{}',
    trigger_when VARCHAR(32) NOT NULL DEFAULT 'first_word', -- first_word, any
    callback_urls TEXT[] NOT NULL,
    content_type VARCHAR(64) DEFAULT 'application/json',
    token VARCHAR(64) NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Slash commands
CREATE TABLE slash_commands (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    trigger VARCHAR(64) NOT NULL,
    url TEXT NOT NULL,
    method VARCHAR(8) NOT NULL DEFAULT 'POST',
    display_name VARCHAR(255),
    description TEXT,
    hint TEXT,
    icon_url TEXT,
    token VARCHAR(64) NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    auto_complete BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT slash_commands_trigger_team_unique UNIQUE (team_id, trigger)
);

-- Bot accounts
CREATE TABLE bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    icon_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Bot access tokens
CREATE TABLE bot_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    token VARCHAR(64) NOT NULL UNIQUE,
    description VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_incoming_webhooks_team ON incoming_webhooks(team_id);
CREATE INDEX idx_incoming_webhooks_channel ON incoming_webhooks(channel_id);
CREATE INDEX idx_incoming_webhooks_token ON incoming_webhooks(token);
CREATE INDEX idx_outgoing_webhooks_team ON outgoing_webhooks(team_id);
CREATE INDEX idx_outgoing_webhooks_channel ON outgoing_webhooks(channel_id);
CREATE INDEX idx_slash_commands_team ON slash_commands(team_id);
CREATE INDEX idx_slash_commands_trigger ON slash_commands(trigger);
CREATE INDEX idx_bots_owner ON bots(owner_id);
CREATE INDEX idx_bot_tokens_bot ON bot_tokens(bot_id);
CREATE INDEX idx_bot_tokens_token ON bot_tokens(token);

-- Triggers
CREATE TRIGGER update_incoming_webhooks_updated_at
    BEFORE UPDATE ON incoming_webhooks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_outgoing_webhooks_updated_at
    BEFORE UPDATE ON outgoing_webhooks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_slash_commands_updated_at
    BEFORE UPDATE ON slash_commands
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_bots_updated_at
    BEFORE UPDATE ON bots
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
