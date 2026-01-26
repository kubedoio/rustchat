-- Channel sidebar categories for Mattermost compatibility

CREATE TABLE channel_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type VARCHAR(32) NOT NULL DEFAULT 'custom',
    display_name VARCHAR(64) NOT NULL,
    sorting VARCHAR(32) NOT NULL DEFAULT 'alpha',
    muted BOOLEAN NOT NULL DEFAULT FALSE,
    collapsed BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INT NOT NULL DEFAULT 0,
    create_at BIGINT NOT NULL,
    update_at BIGINT NOT NULL,
    delete_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE channel_category_channels (
    category_id UUID NOT NULL REFERENCES channel_categories(id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sort_order INT NOT NULL DEFAULT 0,
    PRIMARY KEY (category_id, channel_id)
);

CREATE INDEX idx_channel_categories_user_team ON channel_categories(user_id, team_id);
