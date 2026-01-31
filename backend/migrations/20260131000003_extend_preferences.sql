-- Increase column sizes for mattermost_preferences to handle longer category and name strings
ALTER TABLE mattermost_preferences ALTER COLUMN category TYPE VARCHAR(128);
ALTER TABLE mattermost_preferences ALTER COLUMN name TYPE VARCHAR(128);
