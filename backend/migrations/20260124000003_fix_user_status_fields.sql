-- Migration: Fix user status fields
-- 20260124000003_fix_user_status_fields.sql

ALTER TABLE users
ADD COLUMN IF NOT EXISTS status_text TEXT,
ADD COLUMN IF NOT EXISTS status_emoji VARCHAR(32),
ADD COLUMN IF NOT EXISTS status_expires_at TIMESTAMPTZ;
