-- Migration: Add unread messages support
-- 20260124000002_unread_messages.sql

-- 1. Add monotonic seq to posts
-- We use BIGSERIAL to ensure each post has a monotonic integer ID.
ALTER TABLE posts ADD COLUMN seq BIGSERIAL;
CREATE INDEX idx_posts_seq ON posts(seq);

-- 2. Create channel_reads table
-- This table stores the last read position for each user in each channel.
CREATE TABLE channel_reads (
  user_id      UUID      NOT NULL,
  channel_id   UUID      NOT NULL,
  last_read_message_id BIGINT,         -- nullable: null = never read (corresponds to posts.seq)
  last_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, channel_id),
  FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
  FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE
);

CREATE INDEX idx_channel_reads_user_id ON channel_reads(user_id);
