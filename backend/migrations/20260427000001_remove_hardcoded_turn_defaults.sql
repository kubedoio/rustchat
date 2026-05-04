-- Remove hardcoded maintainer TURN server defaults from calls plugin config
-- Previous migration embedded turn.kubedo.io infrastructure and static credentials.
-- This migration makes TURN opt-in and clears any lingering hardcoded defaults.

-- Update rows that still have the old hardcoded TURN server URL
UPDATE server_config
SET plugins = jsonb_set(
    plugins,
    '{calls}',
    '{
        "enabled": true,
        "turn_server_enabled": false,
        "turn_server_url": "",
        "turn_server_username": "",
        "turn_server_credential": "",
        "udp_port": 8443,
        "tcp_port": 8443,
        "ice_host_override": null,
        "stun_servers": ["stun:stun.l.google.com:19302"]
    }'::jsonb
)
WHERE plugins->'calls'->>'turn_server_url' = 'turn:turn.kubedo.io:3478';

-- Update the column default for future rows
ALTER TABLE server_config
ALTER COLUMN plugins SET DEFAULT '{
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
