-- Audio/Video Calls Schema Migration

-- Calls table (active and historic sessions)
CREATE TABLE IF NOT EXISTS calls (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID REFERENCES channels(id) ON DELETE CASCADE,
    
    -- If it's a DM, channel_id might be enough, but for optimization we can track type
    type VARCHAR(20) DEFAULT 'audio', -- audio, video, screen
    
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    
    owner_id UUID REFERENCES users(id), -- Who started it
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Call participants
CREATE TABLE IF NOT EXISTS call_participants (
    call_id UUID NOT NULL REFERENCES calls(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    role VARCHAR(20) DEFAULT 'attendee', -- host, attendee
    
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    left_at TIMESTAMP WITH TIME ZONE,
    
    -- State
    muted BOOLEAN DEFAULT false,
    raised_hand BOOLEAN DEFAULT false,
    
    PRIMARY KEY(call_id, user_id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_calls_channel ON calls(channel_id);
CREATE INDEX IF NOT EXISTS idx_calls_active ON calls(ended_at) WHERE ended_at IS NULL;
