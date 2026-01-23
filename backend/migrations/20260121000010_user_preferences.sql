-- User Status and Preferences Migration
-- Adds custom status, notification preferences, and display settings

-- Add status fields to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS status_text VARCHAR(100);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status_emoji VARCHAR(10);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status_expires_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS timezone VARCHAR(64) DEFAULT 'UTC';
ALTER TABLE users ADD COLUMN IF NOT EXISTS position VARCHAR(128);
ALTER TABLE users ADD COLUMN IF NOT EXISTS phone VARCHAR(32);
ALTER TABLE users ADD COLUMN IF NOT EXISTS locale VARCHAR(10) DEFAULT 'en';

-- Create user_preferences table for detailed settings
CREATE TABLE IF NOT EXISTS user_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    
    -- Notification preferences
    notify_desktop VARCHAR(20) DEFAULT 'all', -- 'all', 'mentions', 'none'
    notify_push VARCHAR(20) DEFAULT 'all',
    notify_email VARCHAR(20) DEFAULT 'none',
    notify_sounds BOOLEAN DEFAULT true,
    
    -- Do Not Disturb
    dnd_enabled BOOLEAN DEFAULT false,
    dnd_start_time TIME,
    dnd_end_time TIME,
    dnd_days VARCHAR(20) DEFAULT '12345', -- Days of week: 1=Mon, 7=Sun
    
    -- Display preferences
    message_display VARCHAR(20) DEFAULT 'standard', -- 'standard', 'compact'
    sidebar_behavior VARCHAR(20) DEFAULT 'unreads_first',
    time_format VARCHAR(10) DEFAULT '12h', -- '12h', '24h'
    
    -- Keywords
    mention_keywords TEXT[], -- Additional keywords that trigger notifications
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create per-channel notification settings
CREATE TABLE IF NOT EXISTS channel_notification_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    
    notify_level VARCHAR(20) DEFAULT 'default', -- 'default', 'all', 'mentions', 'none'
    is_muted BOOLEAN DEFAULT false,
    mute_until TIMESTAMP WITH TIME ZONE,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(user_id, channel_id)
);

-- Status presets table
CREATE TABLE IF NOT EXISTS status_presets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE, -- NULL for system presets
    emoji VARCHAR(10) NOT NULL,
    text VARCHAR(100) NOT NULL,
    duration_minutes INTEGER, -- NULL for manual clear
    is_default BOOLEAN DEFAULT false,
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Insert default status presets
INSERT INTO status_presets (emoji, text, duration_minutes, is_default, sort_order) VALUES
    ('📅', 'In a meeting', 60, true, 1),
    ('🚗', 'Commuting', 30, true, 2),
    ('🤒', 'Out sick', NULL, true, 3),
    ('🌴', 'On vacation', NULL, true, 4),
    ('🏠', 'Working remotely', NULL, true, 5)
ON CONFLICT DO NOTHING;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_user_preferences_user_id ON user_preferences(user_id);
CREATE INDEX IF NOT EXISTS idx_channel_notification_settings_user ON channel_notification_settings(user_id);
CREATE INDEX IF NOT EXISTS idx_channel_notification_settings_channel ON channel_notification_settings(channel_id);
CREATE INDEX IF NOT EXISTS idx_status_presets_user ON status_presets(user_id);
