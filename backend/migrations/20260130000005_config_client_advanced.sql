-- Extend server_config defaults for client config fields

ALTER TABLE server_config
    ALTER COLUMN site SET DEFAULT '{
        "site_name": "RustChat",
        "site_description": "A self-hosted team collaboration platform",
        "site_url": "",
        "about_link": "https://docs.mattermost.com/about/product.html/",
        "help_link": "https://mattermost.com/default-help/",
        "terms_of_service_link": "https://about.mattermost.com/default-terms/",
        "privacy_policy_link": "",
        "report_a_problem_link": "https://mattermost.com/default-report-a-problem/",
        "support_email": "",
        "app_download_link": "https://mattermost.com/download/#mattermostApps",
        "android_app_download_link": "https://mattermost.com/mattermost-android-app/",
        "ios_app_download_link": "https://mattermost.com/mattermost-ios-app/",
        "custom_brand_text": "",
        "custom_description_text": "",
        "service_environment": "production",
        "max_file_size_mb": 50,
        "max_simultaneous_connections": 5,
        "enable_file": true,
        "enable_user_statuses": true,
        "enable_custom_emoji": true,
        "enable_custom_brand": false,
        "enable_mobile_file_download": true,
        "enable_mobile_file_upload": true,
        "allow_download_logs": true,
        "diagnostics_enabled": false,
        "default_locale": "en",
        "default_timezone": "UTC"
    }'::jsonb;

ALTER TABLE server_config
    ALTER COLUMN authentication SET DEFAULT '{
        "enable_email_password": true,
        "enable_sso": false,
        "require_sso": false,
        "allow_registration": true,
        "enable_sign_in_with_email": true,
        "enable_sign_in_with_username": true,
        "enable_sign_up_with_email": true,
        "enable_sign_up_with_gitlab": false,
        "enable_sign_up_with_google": false,
        "enable_sign_up_with_office365": false,
        "enable_sign_up_with_openid": false,
        "enable_user_creation": true,
        "enable_open_server": false,
        "enable_guest_accounts": false,
        "enable_multifactor_authentication": false,
        "enforce_multifactor_authentication": false,
        "enable_saml": false,
        "enable_ldap": false,
        "password_min_length": 8,
        "password_require_lowercase": true,
        "password_require_uppercase": true,
        "password_require_number": true,
        "password_require_symbol": false,
        "password_enable_forgot_link": true,
        "session_length_hours": 24
    }'::jsonb;

UPDATE server_config
SET site = jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
        site,
        '{about_link}', to_jsonb('https://docs.mattermost.com/about/product.html/'::text), true
    ),
        '{help_link}', to_jsonb('https://mattermost.com/default-help/'::text), true
    ),
        '{terms_of_service_link}', to_jsonb('https://about.mattermost.com/default-terms/'::text), true
    ),
        '{privacy_policy_link}', to_jsonb(''::text), true
    ),
        '{report_a_problem_link}', to_jsonb('https://mattermost.com/default-report-a-problem/'::text), true
    ),
        '{support_email}', to_jsonb(''::text), true
    ),
        '{app_download_link}', to_jsonb('https://mattermost.com/download/#mattermostApps'::text), true
    ),
        '{android_app_download_link}', to_jsonb('https://mattermost.com/mattermost-android-app/'::text), true
    ),
        '{ios_app_download_link}', to_jsonb('https://mattermost.com/mattermost-ios-app/'::text), true
    ),
        '{custom_brand_text}', to_jsonb(''::text), true
    ),
        '{custom_description_text}', to_jsonb(''::text), true
    ),
        '{service_environment}', to_jsonb('production'::text), true
    ),
        '{enable_file}', to_jsonb(true), true
    ),
        '{enable_user_statuses}', to_jsonb(true), true
    ),
        '{enable_custom_emoji}', to_jsonb(true), true
    ),
        '{enable_custom_brand}', to_jsonb(false), true
    ),
        '{enable_mobile_file_download}', to_jsonb(true), true
    ),
        '{enable_mobile_file_upload}', to_jsonb(true), true
    ),
        '{allow_download_logs}', to_jsonb(true), true
    ),
        '{diagnostics_enabled}', to_jsonb(false), true
    )
WHERE id = 'default';

UPDATE server_config
SET authentication = jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
    jsonb_set(
        authentication,
        '{enable_sign_in_with_email}', to_jsonb(true), true
    ),
        '{enable_sign_in_with_username}', to_jsonb(true), true
    ),
        '{enable_sign_up_with_email}', to_jsonb(true), true
    ),
        '{enable_sign_up_with_gitlab}', to_jsonb(false), true
    ),
        '{enable_sign_up_with_google}', to_jsonb(false), true
    ),
        '{enable_sign_up_with_office365}', to_jsonb(false), true
    ),
        '{enable_sign_up_with_openid}', to_jsonb(false), true
    ),
        '{enable_user_creation}', to_jsonb(true), true
    ),
        '{enable_open_server}', to_jsonb(false), true
    ),
        '{enable_guest_accounts}', to_jsonb(false), true
    ),
        '{enable_multifactor_authentication}', to_jsonb(false), true
    ),
        '{enforce_multifactor_authentication}', to_jsonb(false), true
    ),
        '{enable_saml}', to_jsonb(false), true
    ),
        '{enable_ldap}', to_jsonb(false), true
    ),
        '{password_require_lowercase}', to_jsonb(true), true
    ),
        '{password_enable_forgot_link}', to_jsonb(true), true
    )
WHERE id = 'default';
