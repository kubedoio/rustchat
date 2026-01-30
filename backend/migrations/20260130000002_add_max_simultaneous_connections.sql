-- Add max_simultaneous_connections to server_config defaults

ALTER TABLE server_config
    ALTER COLUMN site SET DEFAULT '{
        "site_name": "RustChat",
        "site_description": "A self-hosted team collaboration platform",
        "site_url": "",
        "max_file_size_mb": 50,
        "max_simultaneous_connections": 5,
        "default_locale": "en",
        "default_timezone": "UTC"
    }'::jsonb;

UPDATE server_config
SET site = jsonb_set(site, '{max_simultaneous_connections}', '5', true)
WHERE id = 'default'
  AND NOT (site ? 'max_simultaneous_connections');
