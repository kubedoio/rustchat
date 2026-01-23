-- Server Configuration Table
-- Stores runtime-configurable settings in structured JSONB columns

CREATE TABLE IF NOT EXISTS server_config (
    id VARCHAR(255) PRIMARY KEY DEFAULT 'default',
    
    -- Site settings
    site JSONB NOT NULL DEFAULT '{
        "site_name": "RustChat",
        "site_description": "A self-hosted team collaboration platform",
        "site_url": "",
        "max_file_size_mb": 50,
        "default_locale": "en",
        "default_timezone": "UTC"
    }'::jsonb,
    
    -- Authentication settings
    authentication JSONB NOT NULL DEFAULT '{
        "enable_email_password": true,
        "enable_sso": false,
        "require_sso": false,
        "allow_registration": true,
        "password_min_length": 8,
        "password_require_uppercase": true,
        "password_require_number": true,
        "password_require_symbol": false,
        "session_length_hours": 24
    }'::jsonb,
    
    -- Integration settings
    integrations JSONB NOT NULL DEFAULT '{
        "enable_webhooks": true,
        "enable_slash_commands": true,
        "enable_bots": true,
        "max_webhooks_per_team": 10,
        "webhook_payload_size_kb": 100
    }'::jsonb,
    
    -- Compliance settings
    compliance JSONB NOT NULL DEFAULT '{
        "message_retention_days": 0,
        "file_retention_days": 0
    }'::jsonb,
    
    -- Email/SMTP settings
    email JSONB NOT NULL DEFAULT '{
        "smtp_host": "",
        "smtp_port": 587,
        "smtp_username": "",
        "smtp_password_encrypted": "",
        "smtp_tls": true,
        "from_address": "",
        "from_name": "RustChat"
    }'::jsonb,
    
    -- Experimental feature flags
    experimental JSONB NOT NULL DEFAULT '{}'::jsonb,
    
    -- Metadata
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by UUID REFERENCES users(id)
);

-- Insert default config row
INSERT INTO server_config (id) VALUES ('default') ON CONFLICT (id) DO NOTHING;

-- Index for quick lookups (though there's typically only one row)
CREATE INDEX IF NOT EXISTS idx_server_config_updated ON server_config(updated_at);
