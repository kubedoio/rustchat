-- Mattermost Compatibility Tables

-- Preferences table for generic key-value storage used by Mattermost clients
CREATE TABLE IF NOT EXISTS mattermost_preferences (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category VARCHAR(32) NOT NULL,
    name VARCHAR(64) NOT NULL,
    value TEXT,
    PRIMARY KEY (user_id, category, name)
);

-- User devices for push notifications
CREATE TABLE IF NOT EXISTS user_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id VARCHAR(255) NOT NULL, -- Client-provided unique device ID
    token VARCHAR(255), -- Push notification token (APNS/FCM)
    platform VARCHAR(32), -- android/ios
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, device_id)
);

CREATE INDEX idx_user_devices_user_id ON user_devices(user_id);

-- Reactions table (if not exists)
CREATE TABLE IF NOT EXISTS reactions (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    emoji_name VARCHAR(64) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (user_id, post_id, emoji_name)
);

CREATE INDEX IF NOT EXISTS idx_reactions_post_id ON reactions(post_id);
