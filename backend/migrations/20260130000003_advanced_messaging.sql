-- Advanced Messaging Migration
-- Migration: 003_advanced_messaging

-- Scheduled Posts
CREATE TABLE IF NOT EXISTS scheduled_posts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    root_id UUID REFERENCES posts(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    props JSONB DEFAULT '{}',
    file_ids UUID[] DEFAULT '{}',
    scheduled_at TIMESTAMPTZ NOT NULL,
    state VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, sent, failed, cancelled
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Post Reminders
CREATE TABLE IF NOT EXISTS post_reminders (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    target_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (user_id, post_id)
);

-- Indexes
CREATE INDEX idx_scheduled_posts_user ON scheduled_posts(user_id);
CREATE INDEX idx_scheduled_posts_at ON scheduled_posts(scheduled_at) WHERE state = 'pending';
CREATE INDEX idx_post_reminders_target ON post_reminders(target_at);

-- Trigger for updated_at
CREATE TRIGGER update_scheduled_posts_updated_at
    BEFORE UPDATE ON scheduled_posts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
