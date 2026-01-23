-- Add presence column to users table
ALTER TABLE users ADD COLUMN presence VARCHAR(20) NOT NULL DEFAULT 'online';
