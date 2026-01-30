-- Thread memberships: track user follow/read status for threads
-- Migration: 20260130000001_add_thread_memberships

CREATE TABLE IF NOT EXISTS thread_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE, -- root post id
    following BOOLEAN NOT NULL DEFAULT true,
    last_read_at TIMESTAMPTZ,
    mention_count INT NOT NULL DEFAULT 0,
    unread_replies_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, post_id)
);

-- Index for fetching user's threads
CREATE INDEX IF NOT EXISTS idx_thread_memberships_user ON thread_memberships(user_id, following);
CREATE INDEX IF NOT EXISTS idx_thread_memberships_post ON thread_memberships(post_id);
