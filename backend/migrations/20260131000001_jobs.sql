-- Jobs tracking table
-- Migration: 20260131000001_jobs

CREATE TABLE IF NOT EXISTS jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    type VARCHAR(64) NOT NULL,  -- 'data_retention', 'export', 'import', etc.
    status VARCHAR(32) NOT NULL DEFAULT 'pending',  -- pending, running, success, error, cancelled
    priority INTEGER NOT NULL DEFAULT 0,
    data JSONB NOT NULL DEFAULT '{}',
    progress INTEGER DEFAULT 0,  -- 0-100
    last_activity_at TIMESTAMPTZ,
    start_at TIMESTAMPTZ,  -- scheduled start time
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient job queries
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_type ON jobs(type);
CREATE INDEX IF NOT EXISTS idx_jobs_created ON jobs(created_at DESC);

-- Updated at trigger
CREATE TRIGGER update_jobs_updated_at
    BEFORE UPDATE ON jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Server logs table (in-memory buffer backed by DB)
CREATE TABLE IF NOT EXISTS server_logs (
    id BIGSERIAL PRIMARY KEY,
    level VARCHAR(10) NOT NULL,  -- DEBUG, INFO, WARN, ERROR
    message TEXT NOT NULL,
    source VARCHAR(128),
    caller VARCHAR(256),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for log queries
CREATE INDEX IF NOT EXISTS idx_server_logs_timestamp ON server_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_server_logs_level ON server_logs(level);

-- Auto-cleanup old logs (keep 7 days)
CREATE OR REPLACE FUNCTION cleanup_old_logs() RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM server_logs WHERE timestamp < NOW() - INTERVAL '7 days';
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER cleanup_server_logs
    AFTER INSERT ON server_logs
    FOR EACH STATEMENT
    WHEN (pg_trigger_depth() = 0)
    EXECUTE FUNCTION cleanup_old_logs();
