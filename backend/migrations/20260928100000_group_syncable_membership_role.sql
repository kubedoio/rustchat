-- Role conferred by the syncable that recorded this tracking row.
--
-- The effective role of a (target, user) membership is the UNION of all
-- active syncable grants (admin if any grant confers admin), so overlapping
-- syncables converge order-independently instead of last-writer-wins.
ALTER TABLE group_syncable_memberships
    ADD COLUMN IF NOT EXISTS role VARCHAR(16) NOT NULL DEFAULT 'member';

ALTER TABLE group_syncable_memberships
    ADD CONSTRAINT group_syncable_memberships_role_check CHECK (role IN ('member', 'admin'));
