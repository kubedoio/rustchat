-- Migration: Add client_msg_id to posts for idempotency

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS client_msg_id VARCHAR(64);

-- Unique partial index: prevents duplicate client_msg_id per user for non-null values
CREATE UNIQUE INDEX IF NOT EXISTS idx_posts_client_msg_id_unique
    ON posts(user_id, client_msg_id)
    WHERE client_msg_id IS NOT NULL;
