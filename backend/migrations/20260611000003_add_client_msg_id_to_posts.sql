-- Migration: Add client_msg_id to posts for idempotency

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS client_msg_id VARCHAR(64);

CREATE INDEX IF NOT EXISTS idx_posts_client_msg_id ON posts(user_id, client_msg_id);
