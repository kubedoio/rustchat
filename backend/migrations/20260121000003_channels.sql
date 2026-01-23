-- Channels and messaging tables
-- Migration: 003_channels

-- Channel types enum
CREATE TYPE channel_type AS ENUM ('public', 'private', 'direct', 'group');

-- Channels table
CREATE TABLE channels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    type channel_type NOT NULL DEFAULT 'public',
    name VARCHAR(64) NOT NULL,
    display_name VARCHAR(255),
    purpose TEXT,
    header TEXT,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    creator_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT channels_name_team_unique UNIQUE (team_id, name)
);

-- Channel members
CREATE TABLE channel_members (
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(32) NOT NULL DEFAULT 'member',
    notify_props JSONB DEFAULT '{}',
    last_viewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (channel_id, user_id)
);

-- Posts (messages)
CREATE TABLE posts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    root_post_id UUID REFERENCES posts(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    props JSONB DEFAULT '{}',
    file_ids UUID[] DEFAULT '{}',
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

-- Reactions
CREATE TABLE reactions (
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji_name VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (post_id, user_id, emoji_name)
);

-- Indexes
CREATE INDEX idx_channels_team_id ON channels(team_id);
CREATE INDEX idx_channels_type ON channels(type);
CREATE INDEX idx_channel_members_user_id ON channel_members(user_id);
CREATE INDEX idx_posts_channel_id_created ON posts(channel_id, created_at DESC);
CREATE INDEX idx_posts_root_post_id ON posts(root_post_id);
CREATE INDEX idx_posts_user_id ON posts(user_id);
CREATE INDEX idx_reactions_post_id ON reactions(post_id);

-- Triggers
CREATE TRIGGER update_channels_updated_at
    BEFORE UPDATE ON channels
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
