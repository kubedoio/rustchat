-- Add reply counting and thread tracking to posts matches Slack data model
-- Migration: 20260123000001_add_thread_fields

ALTER TABLE posts
ADD COLUMN reply_count INT NOT NULL DEFAULT 0,
ADD COLUMN last_reply_at TIMESTAMPTZ;

-- Index for fetching thread replies efficiently (by root_id)
CREATE INDEX idx_posts_root_post_id_created ON posts(root_post_id, created_at);

-- Index for sorting channels by activity (if we used last_post_at, but here we likely query posts by channel and created)
-- Existing idx_posts_channel_id_created covers the main feed.
