-- Buzz channel mappings: RustChat-authoritative ownership.
--
-- A mapping says "RustChat channel X bridges (outbound) to Buzz channel Y on
-- connection C". RustChat owns the mapping table; the Buzz side only needs
-- the bridge pubkey to be a member of the Buzz channel (added by a Buzz
-- admin with a kind 9000 add-member event). One RustChat channel may bridge
-- to at most one Buzz channel per connection and vice versa.

CREATE TABLE IF NOT EXISTS buzz_channel_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    connection_id UUID NOT NULL REFERENCES buzz_connections(id) ON DELETE CASCADE,
    rustchat_channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    -- Buzz channel id (UUID used in the Nostr 'h' tag).
    buzz_channel_id UUID NOT NULL,
    outbound_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (connection_id, rustchat_channel_id),
    UNIQUE (connection_id, buzz_channel_id)
);

-- Enqueue lookup: find mappings for a RustChat channel.
CREATE INDEX IF NOT EXISTS idx_buzz_channel_mappings_rustchat
    ON buzz_channel_mappings (rustchat_channel_id)
    WHERE outbound_enabled;
