-- Migration: Add entity model support to users table
-- Backward compatible: all existing users default to entity_type='human'

-- Add entity columns
ALTER TABLE users ADD COLUMN IF NOT EXISTS entity_type VARCHAR(32) NOT NULL DEFAULT 'human';
ALTER TABLE users ADD COLUMN IF NOT EXISTS api_key_hash VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS entity_metadata JSONB DEFAULT '{}';
ALTER TABLE users ADD COLUMN IF NOT EXISTS rate_limit_tier VARCHAR(32) DEFAULT 'human_standard';

-- Add CHECK constraint for entity_type (idempotent)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_entity_type_check'
    ) THEN
        ALTER TABLE users ADD CONSTRAINT users_entity_type_check
            CHECK (entity_type IN ('human', 'agent', 'service', 'ci'));
    END IF;
END $$;

-- Add CHECK constraint for rate_limit_tier (idempotent)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_rate_limit_tier_check'
    ) THEN
        ALTER TABLE users ADD CONSTRAINT users_rate_limit_tier_check
            CHECK (rate_limit_tier IN ('human_standard', 'agent_high', 'service_unlimited', 'ci_standard'));
    END IF;
END $$;

-- Create indexes for new columns
CREATE INDEX IF NOT EXISTS idx_users_entity_type ON users(entity_type) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_users_api_key ON users(api_key_hash) WHERE api_key_hash IS NOT NULL;

-- Migrate existing bots to 'agent' entity_type
UPDATE users SET entity_type = 'agent', rate_limit_tier = 'agent_high' WHERE is_bot = true;

-- Comments for documentation
COMMENT ON COLUMN users.entity_type IS 'Type of entity: human, agent, service, or ci';
COMMENT ON COLUMN users.api_key_hash IS 'bcrypt hash of API key for non-human entities';
COMMENT ON COLUMN users.entity_metadata IS 'Flexible JSON metadata specific to entity type';
COMMENT ON COLUMN users.rate_limit_tier IS 'Rate limiting tier for this entity';
