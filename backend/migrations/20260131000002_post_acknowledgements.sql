-- Post acknowledgements for persistent read receipts
CREATE TABLE IF NOT EXISTS post_acknowledgements (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    acknowledged_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, post_id)
);

CREATE INDEX idx_post_acknowledgements_post_id ON post_acknowledgements(post_id);
