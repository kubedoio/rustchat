-- Add Calls Plugin settings to server_config
-- This allows admin console to manage RustChat Calls Plugin settings
-- TURN server is disabled by default; operators must configure their own relay.

-- Add calls_plugin settings to server_config table
ALTER TABLE server_config 
ADD COLUMN IF NOT EXISTS plugins JSONB NOT NULL DEFAULT '{
    "calls": {
        "enabled": true,
        "turn_server_enabled": false,
        "turn_server_url": "",
        "turn_server_username": "",
        "turn_server_credential": "",
        "udp_port": 8443,
        "tcp_port": 8443,
        "ice_host_override": null,
        "stun_servers": ["stun:stun.l.google.com:19302"]
    }
}'::jsonb;

-- Update existing rows to have the default plugins config if they don't have it
UPDATE server_config 
SET plugins = '{
    "calls": {
        "enabled": true,
        "turn_server_enabled": false,
        "turn_server_url": "",
        "turn_server_username": "",
        "turn_server_credential": "",
        "udp_port": 8443,
        "tcp_port": 8443,
        "ice_host_override": null,
        "stun_servers": ["stun:stun.l.google.com:19302"]
    }
}'::jsonb
WHERE plugins IS NULL OR plugins = '{}'::jsonb;
