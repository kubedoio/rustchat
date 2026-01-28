-- Ensure all users have a role and can create posts

UPDATE users
SET role = 'member'
WHERE role IS NULL OR role = '';

INSERT INTO role_permissions (role, permission_id)
SELECT role, 'post.create'
FROM (SELECT DISTINCT role FROM users WHERE role IS NOT NULL AND role <> '') roles
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role, permission_id) VALUES
    ('system_admin', 'post.create'),
    ('org_admin', 'post.create'),
    ('team_admin', 'post.create'),
    ('member', 'post.create'),
    ('guest', 'post.create')
ON CONFLICT DO NOTHING;
