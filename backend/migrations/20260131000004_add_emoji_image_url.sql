-- Add image_url column to custom_emojis table
ALTER TABLE custom_emojis ADD COLUMN IF NOT EXISTS image_url VARCHAR(512);
