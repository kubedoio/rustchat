-- Buzz bridge connections (optional external integration).
--
-- A connection is one remote Buzz relay plus the secp256k1 identity RustChat
-- signs bridge events with. The private key is encrypted at rest with the
-- server encryption key (crypto::encrypt, AES-256-GCM) and is never included
-- in API responses or logs; `bridge_pubkey` is the derived public key (hex)
-- stored for display and for filtering our own events.

CREATE TABLE IF NOT EXISTS buzz_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    -- Base URL of the remote relay HTTP bridge, e.g. https://buzz.example.com
    relay_url TEXT NOT NULL,
    bridge_pubkey CHAR(64) NOT NULL,
    private_key_encrypted TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (name),
    UNIQUE (relay_url)
);
