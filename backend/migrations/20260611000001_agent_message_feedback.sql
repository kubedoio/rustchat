CREATE TABLE IF NOT EXISTS agent_message_feedback (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    feedback_type VARCHAR(8) NOT NULL CHECK (feedback_type IN ('positive', 'negative')),
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_agent_feedback_post ON agent_message_feedback(post_id);
CREATE INDEX IF NOT EXISTS idx_agent_feedback_user ON agent_message_feedback(user_id);
