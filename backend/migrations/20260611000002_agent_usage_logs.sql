CREATE TABLE IF NOT EXISTS agent_usage_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent_configs(user_id),
    channel_id UUID NOT NULL REFERENCES channels(id),
    trigger_type VARCHAR(16) NOT NULL,
    tokens_input INT NOT NULL DEFAULT 0,
    tokens_output INT NOT NULL DEFAULT 0,
    latency_ms INT NOT NULL DEFAULT 0,
    model VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agent_usage_agent ON agent_usage_logs(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_usage_created ON agent_usage_logs(created_at);
