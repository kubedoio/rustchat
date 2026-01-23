-- Increase channel name length to support DM names with multiple UUIDs
-- Migration: 20260122000010_increase_channel_name_length.sql

ALTER TABLE channels ALTER COLUMN name TYPE VARCHAR(255);
