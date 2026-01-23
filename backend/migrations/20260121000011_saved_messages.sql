-- Saved Messages / Bookmarks Migration
-- Allows users to save/bookmark messages for later reference

CREATE TABLE IF NOT EXISTS saved_posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    note TEXT, -- Optional note about why it was saved
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(user_id, post_id)
);

-- Channel bookmarks (pinned links at top of channel)
CREATE TABLE IF NOT EXISTS channel_bookmarks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(100) NOT NULL,
    url TEXT NOT NULL,
    emoji VARCHAR(10),
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    CONSTRAINT unique_channel_bookmark_url UNIQUE(channel_id, url)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_saved_posts_user ON saved_posts(user_id);
CREATE INDEX IF NOT EXISTS idx_saved_posts_post ON saved_posts(post_id);
CREATE INDEX IF NOT EXISTS idx_channel_bookmarks_channel ON channel_bookmarks(channel_id);
