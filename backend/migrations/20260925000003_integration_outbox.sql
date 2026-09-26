-- Integration outbox: durable, retryable delivery of cross-system events.
--
-- Generic over `provider`; phase one is used exclusively by the Buzz bridge
-- (outbound channel bridging). Rows are enqueued in the same transaction as
-- the RustChat state change they describe (transactional outbox pattern) and
-- drained by the integration outbox dispatcher.

CREATE TABLE IF NOT EXISTS integration_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Integration identifier ('buzz'). Fixed vocabulary, not free-form.
    provider VARCHAR(32) NOT NULL,
    -- Owning connection. SET NULL on connection deletion so delivered
    -- history is retained for audit after a connection is removed.
    connection_id UUID REFERENCES buzz_connections(id) ON DELETE SET NULL,
    -- Unique per (provider, idempotency_key); duplicates (e.g. a retried
    -- enqueue after a client retry with the same client_msg_id) are rejected
    -- by the unique constraint rather than delivered twice.
    idempotency_key VARCHAR(255) NOT NULL,
    -- Logical event type ('message.created'). Fixed vocabulary.
    event_type VARCHAR(64) NOT NULL,
    -- Provider-specific payload (JSON). For Buzz: the fully determined
    -- content of the Nostr event to build (post id, channel, content,
    -- author label) — everything needed to rebuild a byte-identical event
    -- on every retry so the Nostr event id (and thus relay-side
    -- deduplication) is stable across attempts.
    payload JSONB NOT NULL,
    -- 'pending' | 'in_flight' | 'delivered' | 'dead_letter'
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    attempts INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error TEXT,
    -- Remote event id (Nostr event id hex) once delivered.
    remote_event_id VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, idempotency_key)
);

-- Dispatch scan: due pending rows, oldest first.
CREATE INDEX IF NOT EXISTS idx_integration_outbox_due
    ON integration_outbox (next_attempt_at, id)
    WHERE status = 'pending';

-- Crash recovery: reclaim in_flight rows whose lease (updated_at) expired.
CREATE INDEX IF NOT EXISTS idx_integration_outbox_in_flight
    ON integration_outbox (updated_at)
    WHERE status = 'in_flight';

-- Delivery history listing per connection.
CREATE INDEX IF NOT EXISTS idx_integration_outbox_connection
    ON integration_outbox (connection_id, created_at DESC);
