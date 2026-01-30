-- Add custom emojis table
-- Migration: 002_custom_emojis

CREATE TABLE custom_emojis (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(64) NOT NULL,
    creator_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    create_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delete_at TIMESTAMPTZ,
    
    CONSTRAINT custom_emojis_name_unique UNIQUE (name)
);

CREATE INDEX idx_custom_emojis_name ON custom_emojis(name);
CREATE INDEX idx_custom_emojis_creator_id ON custom_emojis(creator_id);

-- Trigger for updated_at
CREATE TRIGGER update_custom_emojis_updated_at
    BEFORE UPDATE ON custom_emojis
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
