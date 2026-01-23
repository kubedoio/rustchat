-- Add is_public flag to teams and allow_open_invite to channels
-- This enables users to browse and join public teams and channels

ALTER TABLE teams ADD COLUMN IF NOT EXISTS is_public BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE teams ADD COLUMN IF NOT EXISTS allow_open_invite BOOLEAN NOT NULL DEFAULT true;

-- Add index for public team lookups
CREATE INDEX IF NOT EXISTS idx_teams_is_public ON teams(is_public) WHERE is_public = true;

-- Update existing teams to be public by default
UPDATE teams SET is_public = true, allow_open_invite = true WHERE is_public IS NULL;
