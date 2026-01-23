-- Enterprise features for rustchat
-- Migration: 006_enterprise

-- Audit log table
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    actor_ip VARCHAR(45),
    action VARCHAR(64) NOT NULL,
    target_type VARCHAR(32) NOT NULL,
    target_id UUID,
    old_values JSONB,
    new_values JSONB,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- SSO configuration per organization
CREATE TABLE sso_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    provider VARCHAR(32) NOT NULL, -- oidc, saml
    display_name VARCHAR(255),
    
    -- OIDC settings
    issuer_url TEXT,
    client_id VARCHAR(255),
    client_secret_encrypted TEXT,
    scopes TEXT[] DEFAULT '{openid,profile,email}',
    
    -- SAML settings
    idp_metadata_url TEXT,
    idp_entity_id TEXT,
    
    -- Common
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    auto_provision BOOLEAN NOT NULL DEFAULT TRUE,
    default_role VARCHAR(32) DEFAULT 'member',
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT sso_configs_org_unique UNIQUE (org_id)
);

-- Retention policies
CREATE TABLE retention_policies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    channel_id UUID REFERENCES channels(id) ON DELETE CASCADE,
    
    -- Policy settings
    retention_days INTEGER NOT NULL DEFAULT 365,
    delete_files BOOLEAN NOT NULL DEFAULT FALSE,
    
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Either org_id OR team_id OR channel_id should be set
    CONSTRAINT retention_scope CHECK (
        (org_id IS NOT NULL AND team_id IS NULL AND channel_id IS NULL) OR
        (org_id IS NULL AND team_id IS NOT NULL AND channel_id IS NULL) OR
        (org_id IS NULL AND team_id IS NULL AND channel_id IS NOT NULL)
    )
);

-- Permission definitions
CREATE TABLE permissions (
    id VARCHAR(64) PRIMARY KEY,
    description TEXT,
    category VARCHAR(32)
);

-- Role permission assignments
CREATE TABLE role_permissions (
    role VARCHAR(32) NOT NULL,
    permission_id VARCHAR(64) NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role, permission_id)
);

-- Insert default permissions
INSERT INTO permissions (id, description, category) VALUES
    ('channel.create', 'Create channels', 'channels'),
    ('channel.delete', 'Delete channels', 'channels'),
    ('channel.manage_members', 'Manage channel members', 'channels'),
    ('post.create', 'Create posts', 'messaging'),
    ('post.edit_own', 'Edit own posts', 'messaging'),
    ('post.edit_others', 'Edit other users posts', 'messaging'),
    ('post.delete_own', 'Delete own posts', 'messaging'),
    ('post.delete_others', 'Delete other users posts', 'messaging'),
    ('user.manage', 'Manage users', 'users'),
    ('team.create', 'Create teams', 'teams'),
    ('team.manage', 'Manage teams', 'teams'),
    ('webhook.create', 'Create webhooks', 'integrations'),
    ('webhook.manage', 'Manage webhooks', 'integrations'),
    ('bot.create', 'Create bots', 'integrations'),
    ('bot.manage', 'Manage bots', 'integrations'),
    ('system.admin', 'System administration', 'system');

-- Assign default permissions to roles
INSERT INTO role_permissions (role, permission_id) VALUES
    -- system_admin gets everything
    ('system_admin', 'channel.create'),
    ('system_admin', 'channel.delete'),
    ('system_admin', 'channel.manage_members'),
    ('system_admin', 'post.create'),
    ('system_admin', 'post.edit_own'),
    ('system_admin', 'post.edit_others'),
    ('system_admin', 'post.delete_own'),
    ('system_admin', 'post.delete_others'),
    ('system_admin', 'user.manage'),
    ('system_admin', 'team.create'),
    ('system_admin', 'team.manage'),
    ('system_admin', 'webhook.create'),
    ('system_admin', 'webhook.manage'),
    ('system_admin', 'bot.create'),
    ('system_admin', 'bot.manage'),
    ('system_admin', 'system.admin'),
    -- org_admin
    ('org_admin', 'channel.create'),
    ('org_admin', 'channel.delete'),
    ('org_admin', 'channel.manage_members'),
    ('org_admin', 'post.create'),
    ('org_admin', 'post.edit_own'),
    ('org_admin', 'post.delete_own'),
    ('org_admin', 'user.manage'),
    ('org_admin', 'team.create'),
    ('org_admin', 'team.manage'),
    ('org_admin', 'webhook.create'),
    ('org_admin', 'webhook.manage'),
    ('org_admin', 'bot.create'),
    ('org_admin', 'bot.manage'),
    -- team_admin
    ('team_admin', 'channel.create'),
    ('team_admin', 'channel.manage_members'),
    ('team_admin', 'post.create'),
    ('team_admin', 'post.edit_own'),
    ('team_admin', 'post.delete_own'),
    ('team_admin', 'webhook.create'),
    -- member
    ('member', 'post.create'),
    ('member', 'post.edit_own'),
    ('member', 'post.delete_own'),
    -- guest
    ('guest', 'post.create');

-- Indexes
CREATE INDEX idx_audit_logs_actor ON audit_logs(actor_user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_target ON audit_logs(target_type, target_id);
CREATE INDEX idx_audit_logs_created ON audit_logs(created_at DESC);
CREATE INDEX idx_sso_configs_org ON sso_configs(org_id);
CREATE INDEX idx_retention_policies_org ON retention_policies(org_id);

-- Triggers
CREATE TRIGGER update_sso_configs_updated_at
    BEFORE UPDATE ON sso_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_retention_policies_updated_at
    BEFORE UPDATE ON retention_policies
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
